//! A pinned `path:` layer in the layer cache (spec §5.3 "Cache integrity", §6 "A pinned `path:`
//! layer"): the layer's directory at a ref, as git's own tree and blobs give it.
//!
//! The entry is `tree2@<tree>`. It is written from the tree's blobs (`git cat-file --batch`), each
//! checked against its id in Rust, never by a checkout, so no attributes file, filter or line-end
//! setting changes a byte; it is verified before it is published and on every later read, against
//! `git ls-tree -r`, whatever the lock says. Next to it the `pathref@` index entry names the tree
//! for account resolution, which runs no git.
//!
//! Every failure is a diagnostic of the spec's kinds: a ref that gives no directory is
//! `cannot resolve ref`, a git step that fails is `cannot read '<source>' at <ref>`, a name that is
//! not valid UTF-8 is the §5.3 `layer` error, a changed entry is the `cache` error that says to
//! delete it, and an index that cannot be written is the warning `cache-index`.

use std::collections::BTreeMap;
use std::path::Path;

use alz_core::account::AccountFolders;
use alz_core::cache::{CacheLocation, DirKey, EntryName};
use alz_core::git::{
    BlobId, EntryDifference, FetchFailureLine, ObjectCheck, ObjectContent, ObjectFormat,
    ObjectKind, TreeEntry, TreeListing, verify_entry,
};
use alz_core::layer::{LayerContents, Purpose, ReadLayer, scan_verdict};
use alz_core::lock::{Integrity, LockedLayer, ObjectId, verify};
use alz_core::path::{AbsolutePath, SafeRelPath};
use alz_core::stack::resolution_failure::{
    cache_differs_from_lock, cache_differs_from_tree, cannot_read_pinned, cannot_resolve_ref,
    cannot_store, layer_touches_account_folder, pinned_tree_refused,
};
use alz_core::stack::{GitRef, Source};
use alz_core::{Diagnostic, Level, Stripped};

use crate::fs::{CacheDir, CacheError, Dir, Published, ReadError, TempEntry, WalkError};
use crate::git::{GitFailure, GitRunner, ReadScope};
use crate::resolve::clone::{FetchedLayer, ReadHow};

/// The most bytes read from one file of an entry that the walk of a layer cannot see.
const SKIPPED_FILE_CAP: usize = 64 * 1024 * 1024;

/// The pinned `path:` entry of the stack to resolve.
#[derive(Debug, Clone, Copy)]
pub struct PinnedLayer<'a> {
    /// The source as written, for messages.
    pub source: &'a Source,
    /// The manifest's `ref`: the tag, branch or commit that pins the layer. `layer.lock` records
    /// it as `rev`.
    pub git_ref: &'a GitRef,
    /// The real path of the layer's directory, as the component walk found it. The index entry is
    /// keyed by it.
    pub real_dir: &'a AbsolutePath,
    /// The account folders (spec §9.1). The repository top that git reports is walked against them
    /// before git reads anything there.
    pub folders: &'a AccountFolders,
    /// The lock's entry for this source, when it has one. Its content hash is checked when the
    /// layer is read from the cache for the commit it pins.
    pub locked: Option<&'a LockedLayer>,
}

/// A pinned layer read from the cache, and the warnings that did not stop it.
#[derive(Debug)]
pub struct PinnedRead {
    /// The layer: its handle, files and `.env` files. `rev` is the manifest's ref and `sha` the
    /// commit it resolved to.
    pub layer: FetchedLayer,
    /// The warnings of the resolution, such as an index entry that could not be written.
    pub warnings: Vec<Diagnostic>,
}

/// Resolves pinned `path:` layers into the cache.
#[derive(Debug, Clone, Copy)]
pub struct PinnedTreeReader<'a> {
    cache: &'a CacheDir,
    location: &'a CacheLocation,
    git: &'a GitRunner,
}

/// The outcome of one step, with the diagnostic of its failure.
type Step<T> = Result<T, Diagnostic>;

/// Why an entry was not accepted.
enum Refusal {
    /// A diagnostic that does not depend on where the entry came from.
    Named(Diagnostic),
    /// The entry differs from its tree.
    Differs,
}

/// What reading one entry gives.
type Verified = (LayerContents, Vec<SafeRelPath>);

impl<'a> PinnedTreeReader<'a> {
    /// A reader that uses `cache`, found at `location`, and `git`.
    #[must_use]
    pub const fn new(cache: &'a CacheDir, location: &'a CacheLocation, git: &'a GitRunner) -> Self {
        Self {
            cache,
            location,
            git,
        }
    }

    /// Gives the layer's entry in the cache:
    /// 1. the top level and the prefix of the layer's directory, and the commit of the ref;
    /// 2. the tree at the prefix, which must be listed as a directory;
    /// 3. the files of the tree, judged as a whole (paths, collisions) before anything is written,
    ///    and every blob checked to be present;
    /// 4. an entry `tree2@<tree>` that exists is verified against that listing; otherwise one is
    ///    written from the blobs, verified and published;
    /// 5. the index entry is written;
    /// 6. an entry read from the cache for the commit the lock pins is checked against the lock's
    ///    content hash.
    ///
    /// # Errors
    ///
    /// The diagnostic of the failure, as the module says.
    pub fn resolve(&self, layer: &PinnedLayer<'_>) -> Step<PinnedRead> {
        let directory = Path::new(layer.real_dir.as_str());
        let top = self
            .git
            .show_toplevel(directory)
            .map_err(|failure| Self::before_tree(layer, &failure))?;
        Self::keep_out_of_account_folders(layer, &top)?;
        let prefix = self
            .git
            .show_prefix(directory)
            .map_err(|failure| Self::before_tree(layer, &failure))?;
        let commit = self
            .git
            .rev_parse_commit(&top, ReadScope::PinnedTree, layer.git_ref)
            .map_err(|failure| Self::before_tree(layer, &failure))?
            .ok_or_else(|| Self::unresolvable(layer))?;
        let tree = self.layer_tree(layer, &top, &commit, &prefix)?;
        let entries = self
            .git
            .ls_tree_recursive(&top, &tree)
            .map_err(|failure| Self::read_failure(layer, &failure))?;
        let listing = TreeListing::from_entries(&entries)
            .map_err(|refusal| pinned_tree_refused(refusal, Self::source_text(layer)))?;
        self.require_blobs(layer, &top, &listing)?;
        let (found, mut warnings) = self.entry(layer, &top, &tree, &listing, commit)?;
        warnings.extend(self.write_index(layer, commit, tree)?);
        let name = EntryName::Tree(tree);
        if found.read == ReadHow::Cached
            && let Some(locked) = layer
                .locked
                .filter(|locked| locked.sha.as_ref() == Some(&commit))
            && verify(&found.contents.files(), locked) == Integrity::Differs
        {
            return Err(cache_differs_from_lock(
                &locked.name,
                Self::source_text(layer),
                self.entry_directory(&name),
            ));
        }
        Ok(PinnedRead {
            layer: FetchedLayer {
                dir: found.dir,
                entry: name,
                contents: found.contents,
                env_files: found.env_files,
                sha: commit,
                rev: layer.git_ref.clone(),
                read: found.read,
            },
            warnings,
        })
    }

    /// The top of the repository is where git is run from and what it reads, and `core.worktree`
    /// or a `.git` file can put it anywhere: it is walked, one component at a time, against the
    /// account folders, so a top at one, in one, at the home folder or above it is refused before
    /// git is asked for anything else (spec §6 "Layers in or around an account folder").
    fn keep_out_of_account_folders(layer: &PinnedLayer<'_>, top: &Path) -> Step<()> {
        let path = top
            .to_str()
            .and_then(|text| AbsolutePath::try_from(text).ok())
            .ok_or_else(|| Self::unresolvable(layer))?;
        match Dir::open_with_real_path(&path, layer.folders) {
            Ok(_) => Ok(()),
            Err(WalkError::Refused(_)) => {
                Err(layer_touches_account_folder(Self::source_text(layer)))
            }
            Err(
                WalkError::Missing
                | WalkError::NotADirectory
                | WalkError::TooManyLinks
                | WalkError::Io(_),
            ) => Err(Self::unresolvable(layer)),
        }
    }

    fn source_text(layer: &PinnedLayer<'_>) -> Stripped {
        Stripped::from_untrusted(&layer.source.to_string())
    }

    fn unresolvable(layer: &PinnedLayer<'_>) -> Diagnostic {
        cannot_resolve_ref(layer.git_ref, Self::source_text(layer))
    }

    fn entry_directory(&self, name: &EntryName) -> Stripped {
        Stripped::from_untrusted(&format!(
            "{}/{name}",
            self.location.path().as_str().trim_end_matches('/')
        ))
    }

    /// The line git's failure gives, or the failure's own words: a deadline, for one.
    fn detail(failure: &GitFailure) -> Option<Stripped> {
        match failure {
            GitFailure::Exited { stderr, .. } => FetchFailureLine::from_stderr(stderr),
            GitFailure::Spawn(_)
            | GitFailure::Io(_)
            | GitFailure::TimedOut { .. }
            | GitFailure::OutputTooLarge
            | GitFailure::Unreadable(_) => {
                FetchFailureLine::from_stderr(failure.to_string().as_bytes())
            }
        }
        .map(FetchFailureLine::into_stripped)
    }

    /// A failed git step as `cannot read '<source>' at <ref>: <line>`.
    fn read_failure(layer: &PinnedLayer<'_>, failure: &GitFailure) -> Diagnostic {
        cannot_read_pinned(
            Self::source_text(layer),
            layer.git_ref,
            Self::detail(failure),
        )
    }

    /// A step before the tree is read that git itself refused (not a repository, unknown ref) is
    /// the ref that cannot be resolved; one that did not finish is a read that failed.
    fn before_tree(layer: &PinnedLayer<'_>, failure: &GitFailure) -> Diagnostic {
        match failure {
            GitFailure::Exited { .. } => Self::unresolvable(layer),
            GitFailure::Spawn(_)
            | GitFailure::Io(_)
            | GitFailure::TimedOut { .. }
            | GitFailure::OutputTooLarge
            | GitFailure::Unreadable(_) => Self::read_failure(layer, failure),
        }
    }

    fn missing_file(layer: &PinnedLayer<'_>, path: &SafeRelPath) -> Diagnostic {
        cannot_read_pinned(
            Self::source_text(layer),
            layer.git_ref,
            Some(Stripped::from_untrusted(&format!(
                "the repository does not hold the file '{path}'"
            ))),
        )
    }

    fn cache_failure(&self, layer: &PinnedLayer<'_>, error: CacheError) -> Diagnostic {
        match error {
            CacheError::EntryNotDirectory(_)
            | CacheError::EntryNotOwned(_)
            | CacheError::EntryVanished(_) => error.into_diagnostic(self.location),
            CacheError::TouchesAccountFolder
            | CacheError::NotPrivate
            | CacheError::InsideRepository
            | CacheError::Unusable(_)
            | CacheError::TempEntryReplaced
            | CacheError::Io(_) => cannot_store(
                Self::source_text(layer),
                Stripped::from_untrusted(self.location.path().as_str()),
            ),
        }
    }

    /// Step 2: the tree of the layer's directory. The prefix is empty at the top of the
    /// repository, where the commit's own tree is the layer. Anywhere else `ls-tree` must list the
    /// path as a directory (`040000 tree`), so a link or a submodule, whose recorded id could be
    /// any tree id, is a ref that cannot be resolved.
    fn layer_tree(
        &self,
        layer: &PinnedLayer<'_>,
        top: &Path,
        commit: &ObjectId,
        prefix: &std::ffi::OsStr,
    ) -> Step<ObjectId> {
        let text = prefix.to_str().ok_or_else(|| Self::unresolvable(layer))?;
        let text = text.strip_suffix('/').unwrap_or(text);
        if text.is_empty() {
            return self
                .git
                .rev_parse_tree(top, commit)
                .map_err(|failure| Self::before_tree(layer, &failure))?
                .ok_or_else(|| Self::unresolvable(layer));
        }
        let path = SafeRelPath::try_from(text).map_err(|_unsafe| Self::unresolvable(layer))?;
        let listed = self
            .git
            .ls_tree_at(top, commit, &path)
            .map_err(|failure| Self::before_tree(layer, &failure))?;
        match listed.as_slice() {
            [entry] if is_directory_at(entry, &path) => Ok(entry.id),
            _ => Err(Self::unresolvable(layer)),
        }
    }

    /// Step 3: the repository holds every blob the tree lists, as a blob, so another repository's
    /// tree id never selects an entry whose content this repository lacks.
    fn require_blobs(
        &self,
        layer: &PinnedLayer<'_>,
        top: &Path,
        listing: &TreeListing,
    ) -> Step<()> {
        let checks = self
            .git
            .cat_file_batch_check(top, &listing.blob_ids())
            .map_err(|failure| Self::read_failure(layer, &failure))?;
        let held: BTreeMap<ObjectId, bool> = checks
            .iter()
            .map(|check| match check {
                ObjectCheck::Present { id, kind, .. } => (*id, *kind == ObjectKind::Blob),
                ObjectCheck::Missing { id } => (*id, false),
            })
            .collect();
        listing
            .files()
            .find(|(_, file)| !held.get(&file.blob).copied().unwrap_or(false))
            .map_or(Ok(()), |(path, _)| Err(Self::missing_file(layer, path)))
    }

    /// Step 4: the entry for `tree`, read or written.
    fn entry(
        &self,
        layer: &PinnedLayer<'_>,
        top: &Path,
        tree: &ObjectId,
        listing: &TreeListing,
        commit: ObjectId,
    ) -> Step<(Found, Vec<Diagnostic>)> {
        let name = EntryName::Tree(*tree);
        let format = ObjectFormat::of(&commit);
        let differs =
            || cache_differs_from_tree(Self::source_text(layer), tree, self.entry_directory(&name));
        let existing = |dir: Dir| {
            Self::verified(layer, &dir, listing, format)
                .map(|(contents, env_files)| Found {
                    dir,
                    contents,
                    env_files,
                    read: ReadHow::Cached,
                })
                .map_err(|refusal| match refusal {
                    Refusal::Named(diagnostic) => diagnostic,
                    Refusal::Differs => differs(),
                })
        };
        if let Some(dir) = self
            .cache
            .entry(&name)
            .map_err(|error| self.cache_failure(layer, error))?
        {
            return existing(dir).map(|found| (found, vec![]));
        }
        let temp =
            TempEntry::create(self.cache).map_err(|error| self.cache_failure(layer, error))?;
        self.write_tree(layer, top, &temp, listing, format)?;
        let (contents, env_files) = Self::verified(layer, temp.dir(), listing, format).map_err(
            |refusal| match refusal {
                Refusal::Named(diagnostic) => diagnostic,
                Refusal::Differs => cannot_read_pinned(
                    Self::source_text(layer),
                    layer.git_ref,
                    Some(Stripped::from_untrusted(
                        "the entry written for the tree does not match it",
                    )),
                ),
            },
        )?;
        match temp
            .publish(&name)
            .map_err(|error| self.cache_failure(layer, error))?
        {
            Published::Created(dir) => Ok((
                Found {
                    dir,
                    contents,
                    env_files,
                    read: ReadHow::Fetched,
                },
                vec![],
            )),
            // Another process published it first: it is checked like any entry in the cache.
            Published::Existing(dir) => existing(dir).map(|found| (found, vec![])),
        }
    }

    /// Writes every file of `listing` into `temp` from the tree's own blobs, each checked against
    /// its id.
    fn write_tree(
        &self,
        layer: &PinnedLayer<'_>,
        top: &Path,
        temp: &TempEntry<'_>,
        listing: &TreeListing,
        format: ObjectFormat,
    ) -> Step<()> {
        let ids = listing.blob_ids();
        let contents = self
            .git
            .cat_file_batch(top, &ids)
            .map_err(|failure| Self::read_failure(layer, &failure))?;
        let blobs = Self::blobs(layer, listing, format, ids.len(), contents)?;
        for (path, file) in listing.files() {
            let bytes = blobs
                .get(&file.blob)
                .ok_or_else(|| Self::missing_file(layer, path))?;
            temp.write_file(path, bytes, file.executable)
                .map_err(|error| self.cache_failure(layer, error))?;
        }
        Ok(())
    }

    /// The content of each blob by id, once each has been checked to hash to its id.
    fn blobs(
        layer: &PinnedLayer<'_>,
        listing: &TreeListing,
        format: ObjectFormat,
        asked: usize,
        contents: Vec<ObjectContent>,
    ) -> Step<BTreeMap<ObjectId, Vec<u8>>> {
        let wrong = |what: &str| {
            cannot_read_pinned(
                Self::source_text(layer),
                layer.git_ref,
                Some(Stripped::from_untrusted(what)),
            )
        };
        if contents.len() != asked {
            return Err(wrong("git answered for another number of blobs"));
        }
        contents
            .into_iter()
            .map(|content| match content {
                ObjectContent::Present {
                    id,
                    kind: ObjectKind::Blob,
                    bytes,
                } if BlobId::of(format, &bytes).id() == id => Ok((id, bytes)),
                ObjectContent::Present { .. } => Err(wrong("a blob does not match its id")),
                ObjectContent::Missing { id } => Err(listing
                    .files()
                    .find(|(_, file)| file.blob == id)
                    .map_or_else(
                        || wrong("a blob is missing"),
                        |(path, _)| Self::missing_file(layer, path),
                    )),
            })
            .collect()
    }

    /// Reads an entry and compares it with the tree (spec §6): the walk with each file's blob id,
    /// the files the walk cannot see read by their path, then the check of `alz-core`. The names
    /// are judged first, as in every layer (§5.3 "File names").
    fn verified(
        layer: &PinnedLayer<'_>,
        dir: &Dir,
        listing: &TreeListing,
        format: ObjectFormat,
    ) -> Result<Verified, Refusal> {
        let scan = dir.scan_against_tree(Purpose::Build, format);
        let read: ReadLayer = scan_verdict(Purpose::Build, scan.clone()).map_err(|refusal| {
            refusal
                .file_name_diagnostic(Self::source_text(layer))
                .map_or(Refusal::Differs, Refusal::Named)
        })?;
        // A skipped name that cannot be read as a regular file is absent from the entry, which
        // `verify_entry` reports as a difference; a read that failed for any other reason says
        // nothing about the entry and is a failed read, never a difference.
        let skipped: BTreeMap<SafeRelPath, ObjectId> = listing
            .skipped()
            .map(|(path, _)| match dir.read_file(path, SKIPPED_FILE_CAP) {
                Ok(bytes) => Ok(Some((path.clone(), BlobId::of(format, &bytes).id()))),
                Err(ReadError::Missing | ReadError::NotRegular | ReadError::TooLarge) => Ok(None),
                Err(ReadError::Io(_)) => Err(Refusal::Named(cannot_read_pinned(
                    Self::source_text(layer),
                    layer.git_ref,
                    Some(Stripped::from_untrusted(
                        "a file of the entry cannot be read",
                    )),
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        verify_entry(listing, &scan, &skipped)
            .map_err(|_difference: EntryDifference| Refusal::Differs)?;
        Ok(read.into_parts())
    }

    /// Step 5: the index entry. A folder at its name is the cache error; any other failure is
    /// the warning `cache-index`.
    fn write_index(
        &self,
        layer: &PinnedLayer<'_>,
        commit: ObjectId,
        tree: ObjectId,
    ) -> Step<Vec<Diagnostic>> {
        match self
            .cache
            .write_index(DirKey::of(layer.real_dir), commit, tree)
        {
            Ok(()) => Ok(vec![]),
            Err(error) => {
                let diagnostic = error.into_diagnostic(self.location);
                if diagnostic.level() == Level::Error {
                    Err(diagnostic)
                } else {
                    Ok(vec![diagnostic])
                }
            }
        }
    }
}

/// What reading or writing the entry found.
struct Found {
    dir: Dir,
    contents: LayerContents,
    env_files: Vec<SafeRelPath>,
    read: ReadHow,
}

/// Whether `entry` is the directory at `path`.
fn is_directory_at(entry: &TreeEntry, path: &SafeRelPath) -> bool {
    entry.mode == alz_core::git::EntryMode::Directory && entry.path == path.as_str().as_bytes()
}
