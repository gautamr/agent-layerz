//! Resolving the stack of a repository (spec §6 steps 1 and 2): each entry's directory is found
//! by the source's own rule, read through a directory handle, parsed, and then the stack as a
//! whole is validated.
//!
//! Every entry is resolved even when an earlier one failed, so one run reports every problem. The
//! things an entry may need are made when the first entry needs them and never otherwise: the
//! cache only for a `git:`, `github:` or pinned `path:` entry, and git (with its version check)
//! after the walk has accepted the directory, so a refused layer never has git run on it.

use std::cell::OnceCell;

use alz_core::account::AccountFolders;
use alz_core::cache::{CacheLocation, CacheLocationError};
use alz_core::layer::{LayerMeta, Purpose, ReadLayer, scan_verdict};
use alz_core::lint;
use alz_core::lock::{LayerFiles, Lock, LockedLayer};
use alz_core::path::{AbsolutePath, SafeRelPath};
use alz_core::stack::resolution_failure::{
    env_file_in_layer, layer_directory_not_found, layer_touches_account_folder, layer_unreadable,
};
use alz_core::stack::{
    FailedEntry, Manifest, Pin, ResolvedEntry, ResolvedLayer, Source, SourceDir, StackEntry,
    StackReport, github_origin,
};
use alz_core::yaml::MAX_BYTES;
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped};

use crate::EnvSnapshot;
use crate::fs::{CacheDir, Dir, ReadError, RepositoryPaths, WalkError};
use crate::git::{Deadlines, GitRunner, GitRunnerError};
use crate::resolve::clone::{FetchedLayer, Fetcher, GitLayer};
use crate::resolve::folders::{account_folders, real_path};
use crate::resolve::locate::locate_git_source;
use crate::resolve::pinned_tree::{PinnedLayer, PinnedRead, PinnedTreeReader};

/// Something made once, on the first use. A failure is told to the first user only: the entries
/// that need it afterwards fail without a second report of the same cause.
#[derive(Debug)]
struct OnFirstUse<T>(OnceCell<Option<T>>);

impl<T> OnFirstUse<T> {
    const fn new() -> Self {
        Self(OnceCell::new())
    }

    fn get(&self, make: impl FnOnce() -> Result<T, Diagnostic>) -> Result<&T, Diagnostics> {
        let mut failure = None;
        self.0
            .get_or_init(|| make().map_err(|diagnostic| failure = Some(diagnostic)).ok())
            .as_ref()
            .ok_or_else(|| failure.into_iter().collect())
    }
}

/// The cache root and where it is.
#[derive(Debug)]
struct Cache {
    location: CacheLocation,
    dir: CacheDir,
}

/// What a source gave: the directory of the layer and what was seen in it.
struct Found {
    dir: Dir,
    files: LayerFiles,
    env_files: Vec<SafeRelPath>,
    pin: Option<Pin>,
    warnings: Diagnostics,
}

/// Resolves the entries of a repository's stack.
#[derive(Debug)]
pub struct StackResolver<'a> {
    env: &'a EnvSnapshot,
    repository: AbsolutePath,
    folders: AccountFolders,
    git: OnFirstUse<GitRunner>,
    cache: OnFirstUse<Cache>,
}

impl<'a> StackResolver<'a> {
    /// A resolver for the repository in the working directory, with the account folders of `env`.
    ///
    /// # Errors
    ///
    /// The error `source` when the working directory cannot be found.
    pub fn in_working_directory(env: &'a EnvSnapshot) -> Result<Self, Diagnostic> {
        std::env::current_dir()
            .ok()
            .and_then(|directory| {
                directory
                    .to_str()
                    .and_then(|text| AbsolutePath::try_from(text).ok())
            })
            .map(|repository| Self::at(env, repository))
            .ok_or_else(|| {
                Diagnostic::error(
                    Code::Source,
                    None,
                    Message::new("cannot find the working directory"),
                )
            })
    }

    /// A resolver for the repository at `repository`.
    #[must_use]
    pub fn at(env: &'a EnvSnapshot, repository: AbsolutePath) -> Self {
        Self {
            env,
            repository,
            folders: account_folders(env),
            git: OnFirstUse::new(),
            cache: OnFirstUse::new(),
        }
    }

    /// Resolves every entry of `manifest`'s stack in order, then validates the stack (§6 step 2).
    /// `lock` is what `layer.lock` records: a layer read from the cache or from git is checked
    /// against it, and an unpinned `path:` layer, which is read in place, never is.
    #[must_use]
    pub fn resolve(&self, manifest: &Manifest, lock: Option<&Lock>) -> StackReport {
        StackReport::of(
            manifest
                .stack()
                .iter()
                .map(|entry| self.entry(entry, lock))
                .collect(),
        )
    }

    fn entry(&self, entry: &StackEntry, lock: Option<&Lock>) -> Result<ResolvedEntry, FailedEntry> {
        let text = source_text(entry.source());
        self.layer_of(entry, lock, &text)
            .map_err(|diagnostics| FailedEntry {
                source: text.clone(),
                diagnostics,
            })
    }

    fn layer_of(
        &self,
        entry: &StackEntry,
        lock: Option<&Lock>,
        text: &Stripped,
    ) -> Result<ResolvedEntry, Diagnostics> {
        let locked = lock.and_then(|lock| {
            lock.layers
                .iter()
                .find(|layer| layer.source == *entry.source())
        });
        let found = match (entry.source(), entry.pin()) {
            (Source::Path { dir }, None) => self.working_tree(dir, text),
            (Source::Path { dir }, Some(git_ref)) => {
                self.pinned_tree(entry.source(), dir, git_ref, locked, text)
            }
            (Source::Git { url, git_ref }, _) => {
                let origin = locate_git_source(url, &self.repository, &self.folders)
                    .map_err(|refusal| single(refusal.into_diagnostic(text.clone())))?;
                self.fetch(entry.source(), git_ref, &origin, locked)
            }
            (
                Source::GitHub {
                    owner,
                    repo,
                    git_ref,
                },
                _,
            ) => self.fetch(entry.source(), git_ref, &github_origin(owner, repo), locked),
        }?;
        read_layer(entry.source(), text, found)
    }

    /// The absolute path of a `path:` source: relative to the repository root, as written.
    fn directory(&self, dir: &SourceDir, text: &Stripped) -> Result<AbsolutePath, Diagnostics> {
        let written = dir.as_str();
        let joined = if written.starts_with('/') {
            written.to_owned()
        } else {
            format!(
                "{}/{written}",
                self.repository.as_str().trim_end_matches('/')
            )
        };
        AbsolutePath::try_from(joined.as_str())
            .map_err(|_not_absolute| single(layer_directory_not_found(text.clone())))
    }

    /// An unpinned `path:` layer: the working tree, opened by the component walk and walked with
    /// no budget (§6, §7.13).
    fn working_tree(&self, dir: &SourceDir, text: &Stripped) -> Result<Found, Diagnostics> {
        let path = self.directory(dir, text)?;
        let opened = Dir::open(&path, &self.folders).map_err(|error| walk_failure(&error, text))?;
        let (files, env_files) = scan_verdict(Purpose::Build, opened.scan(Purpose::Build))
            .map(ReadLayer::into_parts)
            .map_err(|refusal| {
                single(
                    refusal
                        .file_name_diagnostic(text.clone())
                        .unwrap_or_else(|| layer_unreadable(text.clone())),
                )
            })?;
        Ok(Found {
            dir: opened,
            files,
            env_files,
            pin: None,
            warnings: Diagnostics::new(),
        })
    }

    /// A pinned `path:` layer: the walk accepts the directory first, then git reads the tree.
    fn pinned_tree(
        &self,
        source: &Source,
        dir: &SourceDir,
        git_ref: &alz_core::stack::GitRef,
        locked: Option<&LockedLayer>,
        text: &Stripped,
    ) -> Result<Found, Diagnostics> {
        let path = self.directory(dir, text)?;
        let (_, real_dir) = Dir::open_with_real_path(&path, &self.folders)
            .map_err(|error| walk_failure(&error, text))?;
        let cache = self.cache()?;
        let git = self.git()?;
        let PinnedRead { layer, warnings } =
            PinnedTreeReader::new(&cache.dir, &cache.location, git)
                .resolve(&PinnedLayer {
                    source,
                    git_ref,
                    real_dir: &real_dir,
                    locked,
                })
                .map_err(single)?;
        Ok(found_in_cache(layer, warnings.into_iter().collect()))
    }

    /// A `git:` or `github:` layer: cloned into the cache once, or read from the entry the lock
    /// pins.
    fn fetch(
        &self,
        source: &Source,
        git_ref: &alz_core::stack::GitRef,
        origin: &alz_core::stack::CloneOrigin,
        locked: Option<&LockedLayer>,
    ) -> Result<Found, Diagnostics> {
        let cache = self.cache()?;
        let git = self.git()?;
        Fetcher::new(&cache.dir, &cache.location, git)
            .resolve(&GitLayer {
                source,
                git_ref,
                origin,
                locked,
            })
            .map(|layer| found_in_cache(layer, Diagnostics::new()))
            .map_err(single)
    }

    fn git(&self) -> Result<&GitRunner, Diagnostics> {
        self.git.get(|| {
            GitRunner::new(self.env, Deadlines::STANDARD).map_err(|error| match error {
                GitRunnerError::Refused(refusal) => refusal.diagnostic(),
                GitRunnerError::Executable(_) => Diagnostic::error(
                    Code::Git,
                    None,
                    Message::new("alz cannot find its own executable, which it needs to run git"),
                ),
            })
        })
    }

    fn cache(&self) -> Result<&Cache, Diagnostics> {
        self.cache.get(|| {
            let location = CacheLocation::from_env(
                self.env.home().and_then(|home| home.to_str()),
                self.env.xdg_cache_home().and_then(|home| home.to_str()),
            )
            .map_err(CacheLocationError::into_diagnostic)?;
            let repository = RepositoryPaths::new(
                self.repository.clone(),
                real_path(&self.repository).unwrap_or_else(|| self.repository.clone()),
            );
            CacheDir::open_or_create(&location, &self.folders, &repository)
                .map(|dir| Cache {
                    location: location.clone(),
                    dir,
                })
                .map_err(|error| error.into_diagnostic(&location))
        })
    }
}

fn single(diagnostic: Diagnostic) -> Diagnostics {
    Diagnostics::new().with(diagnostic)
}

fn source_text(source: &Source) -> Stripped {
    Stripped::from_untrusted(&source.to_string())
}

/// A walk that failed for a `path:` layer: a directory that touches an account folder or the home
/// folder is refused as such (§6), anything else is a directory that is not there.
fn walk_failure(error: &WalkError, text: &Stripped) -> Diagnostics {
    single(match error {
        WalkError::Refused(_) => layer_touches_account_folder(text.clone()),
        WalkError::Missing
        | WalkError::NotADirectory
        | WalkError::TooManyLinks
        | WalkError::Io(_) => layer_directory_not_found(text.clone()),
    })
}

fn found_in_cache(layer: FetchedLayer, warnings: Diagnostics) -> Found {
    Found {
        dir: layer.dir,
        files: layer.files,
        env_files: layer.env_files,
        pin: Some(Pin {
            rev: layer.rev,
            sha: layer.sha,
        }),
        warnings,
    }
}

/// Reads `layer.yaml` of a found layer (§5.1, §5.3): a regular file of at most 1 MiB, never
/// through a link, parsed into the layer's metadata; then every `.env` file is an error.
fn read_layer(
    source: &Source,
    text: &Stripped,
    found: Found,
) -> Result<ResolvedEntry, Diagnostics> {
    let Found {
        dir,
        files,
        env_files,
        pin,
        warnings,
    } = found;
    let failed = |problems: Diagnostics| warnings.clone().merge(problems);
    let bytes = SafeRelPath::try_from("layer.yaml")
        .map_err(|_unsafe| ReadError::Missing)
        .and_then(|path| dir.read_file(&path, MAX_BYTES))
        .map_err(|error| failed(single(error.into_layer_yaml_diagnostic(text))))?;
    let meta: LayerMeta = lint::read_layer(text, lint::parse_document(&bytes)).map_err(failed)?;
    let env_errors: Diagnostics = env_files
        .iter()
        .map(|file| env_file_in_layer(meta.name(), file))
        .collect();
    Ok(ResolvedEntry {
        diagnostics: warnings.merge(env_errors),
        layer: ResolvedLayer::new(source.clone(), meta, files, pin),
    })
}
