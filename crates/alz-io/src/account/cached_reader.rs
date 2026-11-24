//! Reading a stack's account declaration from the layer cache, without git (spec §7.13 "No fetch
//! on resolve"; decision-18; TASK-55.7.8).
//!
//! The reader is given the repository as a [`ReadOnlyDir`] and the cache as a [`CacheView`]. It
//! gets no `GitRunner`, [`RepoRoot`](crate::RepoRoot) or [`StateDir`](crate::account::StateDir),
//! so it cannot fetch, run git or write: that is a fact of its signature, and the tests check it
//! once more. It decides nothing itself. It finds the facts, which `alz-core`'s
//! [`decide`] and [`read_plan`] turn into a declaration or a miss.

use alz_core::account::{
    AccountFolders, CachedDeclaration, CachedLayer, Miss, MissReason, ReadPlan, decide,
    has_parent_segment, lies_within, lock_entry, lock_miss, read_plan,
};
use alz_core::cache::{DirKey, EntryName};
use alz_core::git::CloneTransport;
use alz_core::layer::{LayerContents, LayerMeta, Purpose, scan_verdict};
use alz_core::lint::parse_document;
use alz_core::lock::{Integrity, Lock, verify};
use alz_core::merge::Placement;
use alz_core::path::{AbsolutePath, SafeRelPath};
use alz_core::stack::{
    CloneOrigin, GitRef, Manifest, OriginRefusal, ResolvedLayer, Source, StackEntry, github_origin,
};
use alz_core::yaml::MAX_BYTES;

use crate::fs::{CacheView, Dir, ReadError, ReadOnlyDir, WalkError};
use crate::resolve::{locate_git_source, path_of};

/// The most `layer.lock` may hold.
const MAX_LOCK_BYTES: usize = 8 * 1024 * 1024;

/// Reads the account declaration of the stack of one repository from the cache.
#[derive(Debug)]
pub struct CachedDeclarationReader<'a> {
    repository: &'a ReadOnlyDir,
    root: &'a AbsolutePath,
    folders: &'a AccountFolders,
    cache: &'a CacheView,
}

/// Where a layer is, found with the component walk and never by a name that was not checked.
struct Site {
    placement: Placement,
    /// What `allowed.yaml` records: a real path, or what `git clone` is given.
    recorded: String,
    /// The clone a `git:` or `github:` layer is cached by.
    origin: Option<CloneOrigin>,
    /// The layer's directory and its real path, for a layer that is on this machine.
    directory: Option<(Dir, AbsolutePath)>,
}

impl<'a> CachedDeclarationReader<'a> {
    /// A reader of the repository whose real root is `root`, opened as `repository`.
    ///
    /// # Errors
    ///
    /// [`MissReason::AccountFolder`] when the root lies inside an account folder or contains one.
    pub fn new(
        repository: &'a ReadOnlyDir,
        root: &'a AbsolutePath,
        folders: &'a AccountFolders,
        cache: &'a CacheView,
    ) -> Result<Self, MissReason> {
        if folders.touches(root) {
            Err(MissReason::AccountFolder)
        } else {
            Ok(Self {
                repository,
                root,
                folders,
                cache,
            })
        }
    }

    /// The declaration of `manifest`'s stack, or the first miss, in stack order. It never fails
    /// because of what a repository, a lock or a cache holds.
    #[must_use]
    pub fn read(&self, manifest: &Manifest) -> CachedDeclaration {
        self.layers(manifest)
            .map_or_else(CachedDeclaration::Miss, |layers| decide(&layers))
    }

    fn locator(&self) -> Locator<'a> {
        Locator {
            root: self.root,
            folders: self.folders,
        }
    }

    fn layers(&self, manifest: &Manifest) -> Result<Vec<CachedLayer>, Miss> {
        let lock = self.lock().map_err(Miss::of)?;
        manifest
            .stack()
            .iter()
            .enumerate()
            .map(|(position, entry)| {
                self.layer(&lock, position, entry)
                    .map_err(|reason| Miss::at(reason, position))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|layers| layers.into_iter().flatten().collect())
    }

    fn lock(&self) -> Result<Lock, MissReason> {
        let path =
            SafeRelPath::try_from("layer.lock").map_err(|_unsafe| MissReason::LockMissing)?;
        match self.repository.read_file(&path, MAX_LOCK_BYTES) {
            Err(ReadError::Missing) => Err(MissReason::LockMissing),
            Err(ReadError::NotRegular | ReadError::TooLarge | ReadError::Io(_)) => {
                Err(MissReason::LockUnreadable)
            }
            Ok(bytes) => Lock::parse(&bytes).map_err(|error| lock_miss(&error)),
        }
    }

    /// The layer of one stack entry: `None` for one inside the repository that declares nothing
    /// that could be read.
    fn layer(
        &self,
        lock: &Lock,
        position: usize,
        entry: &StackEntry,
    ) -> Result<Option<CachedLayer>, MissReason> {
        if has_parent_segment(entry.source()) {
            return Err(MissReason::SourceInvalid);
        }
        let site = self.locator().site(entry.source())?;
        match site.placement {
            Placement::Inside => Ok(inside_layer(position, entry, site)),
            Placement::Outside => self.outside_layer(lock, position, entry, site).map(Some),
        }
    }

    fn outside_layer(
        &self,
        lock: &Lock,
        position: usize,
        entry: &StackEntry,
        site: Site,
    ) -> Result<CachedLayer, MissReason> {
        let locked = lock_entry(lock, entry.source())?;
        let plan = read_plan(entry, locked)?;
        let Site {
            recorded,
            origin,
            directory,
            ..
        } = site;
        let dir = match plan {
            ReadPlan::Git(commit) => {
                let origin = origin.ok_or(MissReason::SourceInvalid)?;
                self.cache.entry(&EntryName::Git {
                    key: origin.key(),
                    commit,
                })?
            }
            ReadPlan::Indexed(commit) => {
                let (_, real) = directory.ok_or(MissReason::LayerDirectoryMissing)?;
                let tree = self
                    .cache
                    .index(DirKey::of(&real), commit)
                    .tree_for(commit)?;
                self.cache.entry(&EntryName::Tree(tree))?
            }
            ReadPlan::InPlace => {
                let (dir, _) = directory.ok_or(MissReason::LayerDirectoryMissing)?;
                if !dir.holds_regular_file("layer.yaml") {
                    return Err(MissReason::LayerInvalid);
                }
                dir
            }
        };
        let read = scan_verdict(Purpose::Resolution, dir.scan(Purpose::Resolution))
            .map_err(MissReason::from)?;
        if verify(read.files(), locked) == Integrity::Differs {
            return Err(MissReason::ContentHash);
        }
        if !read.env_files().is_empty() {
            return Err(MissReason::LayerInvalid);
        }
        let (contents, _) = read.into_parts();
        let meta = layer_meta(&contents).ok_or(MissReason::LayerInvalid)?;
        Ok(CachedLayer {
            position,
            placement: Placement::Outside,
            layer: ResolvedLayer::new(entry.source().clone(), meta, contents, None),
            git_ref: written_ref(entry),
            revision: locked.sha,
            recorded_source: recorded,
        })
    }
}

/// A layer on a remote: nothing of it is on this machine but its cache entry.
fn remote(origin: CloneOrigin) -> Site {
    Site {
        placement: Placement::Outside,
        recorded: origin.argument().to_owned(),
        origin: Some(origin),
        directory: None,
    }
}

/// A layer inside the repository is read only to warn when it declares an account, which it may
/// not: its `layer.yaml` alone, in place, as a regular file of at most 1 MiB. It is never checked
/// against the lock, and anything that cannot be read or parsed leaves it out, so editing
/// `.layer/` never changes or blocks the account. A `git:` layer inside is not read.
fn inside_layer(position: usize, entry: &StackEntry, site: Site) -> Option<CachedLayer> {
    let (dir, _) = site.directory.filter(|_| site.origin.is_none())?;
    let path = SafeRelPath::try_from("layer.yaml").ok()?;
    let bytes = dir.read_file(&path, MAX_BYTES).ok()?;
    let meta = parse_document(&bytes)
        .ok()
        .and_then(|document| LayerMeta::from_yaml(&document).ok())?;
    Some(CachedLayer {
        position,
        placement: Placement::Inside,
        layer: ResolvedLayer::new(entry.source().clone(), meta, LayerContents::default(), None),
        git_ref: written_ref(entry),
        revision: None,
        recorded_source: site.recorded,
    })
}

/// The `layer.yaml` the walk read, which the content hash covers: parsed from those bytes and never
/// from a second read. `None` when it is missing, over 1 MiB or not a valid layer.
fn layer_meta(contents: &LayerContents) -> Option<LayerMeta> {
    let path = SafeRelPath::try_from("layer.yaml").ok()?;
    contents
        .get(&path)
        .filter(|bytes| bytes.len() <= MAX_BYTES)
        .and_then(|bytes| parse_document(bytes).ok())
        .and_then(|document| LayerMeta::from_yaml(&document).ok())
}

/// The ref the manifest names for the entry, as written.
fn written_ref(entry: &StackEntry) -> Option<GitRef> {
    match entry.source() {
        Source::Git { git_ref, .. } | Source::GitHub { git_ref, .. } => Some(git_ref.clone()),
        Source::Path { .. } => entry.pin().cloned(),
    }
}

/// Finds where each stack source is, from the repository's root and the account folders alone:
/// it opens no cache and no `layer.lock`.
#[derive(Debug, Clone, Copy)]
struct Locator<'a> {
    root: &'a AbsolutePath,
    folders: &'a AccountFolders,
}

impl Locator<'_> {
    fn site(&self, source: &Source) -> Result<Site, MissReason> {
        match source {
            Source::Path { dir } => {
                let written = path_of(dir.as_str(), self.root).ok_or(MissReason::SourceInvalid)?;
                self.local(&written)
                    .map(|(directory, real, placement)| Site {
                        placement,
                        recorded: real.as_str().to_owned(),
                        origin: None,
                        directory: Some((directory, real)),
                    })
            }
            Source::Git { url, .. } => {
                let origin = locate_git_source(url, self.root, self.folders).map_err(
                    |refusal| match refusal {
                        OriginRefusal::AccountFolder => MissReason::AccountFolder,
                        OriginRefusal::DotDot | OriginRefusal::InvalidFileUrl => {
                            MissReason::SourceInvalid
                        }
                    },
                )?;
                match origin.transport() {
                    CloneTransport::Remote => Ok(remote(origin)),
                    CloneTransport::Local => {
                        let written = AbsolutePath::try_from(origin.argument())
                            .map_err(|_relative| MissReason::SourceInvalid)?;
                        self.local(&written)
                            .map(|(directory, real, placement)| Site {
                                placement,
                                recorded: origin.argument().to_owned(),
                                origin: Some(origin),
                                directory: Some((directory, real)),
                            })
                    }
                }
            }
            Source::GitHub { owner, repo, .. } => Ok(remote(github_origin(owner, repo))),
        }
    }

    /// Opens the directory at `written` by the component walk, and says whether it is inside the
    /// repository, as written or by its real path.
    fn local(&self, written: &AbsolutePath) -> Result<(Dir, AbsolutePath, Placement), MissReason> {
        Dir::open_with_real_path(written, self.folders)
            .map(|(directory, real)| {
                let inside = [written, &real]
                    .into_iter()
                    .any(|path| lies_within(path, self.root));
                let placement = if inside {
                    Placement::Inside
                } else {
                    Placement::Outside
                };
                (directory, real, placement)
            })
            .map_err(|error| match error {
                WalkError::Refused(_) => MissReason::AccountFolder,
                WalkError::Missing | WalkError::NotADirectory | WalkError::TooManyLinks => {
                    MissReason::LayerDirectoryMissing
                }
                WalkError::Io(_) => MissReason::Unreadable,
            })
    }
}

/// The sources of `manifest`'s stack as `allowed.yaml` records them: a local layer's real path, a
/// remote URL without its ref, a local repository's canonical path. A source that is not valid,
/// names an account folder or is a layer directory that does not exist has none. It needs no
/// cache and no lock: whether the recorded source is still in the stack does not depend on them
/// (spec §7.13 "A named account that cannot apply fails closed").
#[must_use]
pub fn recorded_sources(
    root: &AbsolutePath,
    folders: &AccountFolders,
    manifest: &Manifest,
) -> Vec<String> {
    let locator = Locator { root, folders };
    manifest
        .stack()
        .iter()
        .filter(|entry| !has_parent_segment(entry.source()))
        .filter_map(|entry| locator.site(entry.source()).ok())
        .map(|site| site.recorded)
        .collect()
}
