//! A `git:` or `github:` layer in the cache (spec §5.3 "Cache integrity", §6): read from the
//! entry the lock pins, or cloned once into a temporary entry, checked out at the commit and
//! published as `git-<key>@<commit>`.
//!
//! Every failure is a diagnostic of the spec's own kinds: a failed git step is `source`
//! ("A failed fetch"), a name that is not valid UTF-8 is the §5.3 `layer` error, and a failure to
//! read or move what git made is `cache` ("cannot store"), never a fetch failure.

use std::ffi::OsStr;
use std::path::PathBuf;

use alz_core::cache::{CacheLocation, EntryName};
use alz_core::git::FetchFailureLine;
use alz_core::layer::{Purpose, ReadLayer, scan_verdict};
use alz_core::lock::{ContentHash, Integrity, LayerFiles, LockedLayer, ObjectId, verify};
use alz_core::path::SafeRelPath;
use alz_core::stack::resolution_failure::{
    cache_differs_from_clone, cache_differs_from_lock, cannot_fetch, cannot_store,
};
use alz_core::stack::{CloneOrigin, GitRef, Source};
use alz_core::{Diagnostic, Stripped};

use crate::fs::{CacheDir, CacheError, Dir, Published, TempEntry};
use crate::git::{GitFailure, GitRunner, ReadScope};

/// The `github:` or `git:` entry of the stack to resolve.
#[derive(Debug, Clone, Copy)]
pub struct GitLayer<'a> {
    /// The source as written, for messages.
    pub source: &'a Source,
    /// The tag, branch or commit after `@`.
    pub git_ref: &'a GitRef,
    /// What is cloned, and how it is reached.
    pub origin: &'a CloneOrigin,
    /// The lock's entry for this source, when it has one. Its `sha` decides which cached copy is
    /// read, and its content hash is what that copy must match.
    pub locked: Option<&'a LockedLayer>,
}

/// How the layer was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadHow {
    /// Read from an entry that was already in the cache.
    Cached,
    /// Just cloned, and published or compared with the entry that was there.
    Fetched,
}

/// A layer in the cache: where it is, what is in it and what `layer.lock` records for it.
#[derive(Debug)]
pub struct FetchedLayer {
    /// The handle on the cache entry.
    pub dir: Dir,
    /// The name of the entry.
    pub entry: EntryName,
    /// The files the content hash covers.
    pub files: LayerFiles,
    /// The `.env` files that were listed and never opened.
    pub env_files: Vec<SafeRelPath>,
    /// The commit.
    pub sha: ObjectId,
    /// The first 12 characters of the commit: `rev` in `layer.lock`.
    pub rev: GitRef,
    /// Whether the entry was there already.
    pub read: ReadHow,
}

/// Resolves `git:` and `github:` layers into the cache.
#[derive(Debug, Clone, Copy)]
pub struct Fetcher<'a> {
    cache: &'a CacheDir,
    location: &'a CacheLocation,
    git: &'a GitRunner,
}

/// The outcome of one step, with the diagnostic of its failure.
type Step<T> = Result<T, Diagnostic>;

impl<'a> Fetcher<'a> {
    /// A fetcher that uses `cache`, found at `location`, and `git`.
    #[must_use]
    pub const fn new(cache: &'a CacheDir, location: &'a CacheLocation, git: &'a GitRunner) -> Self {
        Self {
            cache,
            location,
            git,
        }
    }

    /// Gives the layer in the cache:
    /// 1. when the lock pins the source and its entry exists, the entry is checked against the
    ///    lock's content hash and read, and git is not run;
    /// 2. otherwise the source is cloned into a temporary entry, `rev-parse` gives the commit
    ///    (of the lock's `sha`, or of the ref) and the commit is checked out;
    /// 3. an entry that exists for that commit is compared with the clone, which is discarded,
    ///    and a difference is the cache error; when there is none, the clone is published.
    ///
    /// # Errors
    ///
    /// The diagnostic of the failure: `source` for a failed clone, `rev-parse` or checkout;
    /// `layer` for a name that is not valid UTF-8; `cache` for an entry that differs from the
    /// lock or from a fresh clone, an entry that is a link, or a clone that cannot be read or
    /// moved into the cache.
    pub fn resolve(&self, layer: &GitLayer<'_>) -> Result<FetchedLayer, Diagnostic> {
        let pinned = layer
            .locked
            .and_then(|locked| locked.sha.map(|sha| (locked, sha)));
        match pinned {
            Some((locked, sha)) => match self.cached(layer, locked, sha)? {
                Some(hit) => Ok(hit),
                None => self.clone_and_publish(layer, Some(sha)),
            },
            None => self.clone_and_publish(layer, None),
        }
    }

    fn source_text(layer: &GitLayer<'_>) -> Stripped {
        Stripped::from_untrusted(&layer.source.to_string())
    }

    fn entry_directory(&self, name: &EntryName) -> Stripped {
        Stripped::from_untrusted(&format!(
            "{}/{name}",
            self.location.path().as_str().trim_end_matches('/')
        ))
    }

    fn cache_root(&self) -> Stripped {
        Stripped::from_untrusted(self.location.path().as_str())
    }

    /// A cache failure as the diagnostic it is: an entry that is a link or vanished is told as
    /// such; anything else is that the clone cannot be stored.
    fn cache_failure(&self, layer: &GitLayer<'_>, error: CacheError) -> Diagnostic {
        match error {
            CacheError::EntryNotDirectory(_) | CacheError::EntryVanished(_) => {
                error.into_diagnostic(self.location)
            }
            CacheError::TouchesAccountFolder
            | CacheError::InsideRepository
            | CacheError::Unusable(_)
            | CacheError::TempEntryReplaced
            | CacheError::Io(_) => cannot_store(Self::source_text(layer), self.cache_root()),
        }
    }

    /// Step 1: the entry the lock pins, checked against the lock, or `None` when there is none.
    fn cached(
        &self,
        layer: &GitLayer<'_>,
        locked: &LockedLayer,
        sha: ObjectId,
    ) -> Step<Option<FetchedLayer>> {
        let name = EntryName::Git {
            key: layer.origin.key(),
            commit: sha,
        };
        let Some(dir) = self
            .cache
            .entry(&name)
            .map_err(|error| self.cache_failure(layer, error))?
        else {
            return Ok(None);
        };
        let (files, env_files) = self.read(layer, &dir)?;
        match verify(&files, locked) {
            Integrity::Matches => Ok(Some(Self::layer(
                dir,
                name,
                files,
                env_files,
                sha,
                ReadHow::Cached,
            ))),
            Integrity::Differs => Err(cache_differs_from_lock(
                &locked.name,
                Self::source_text(layer),
                self.entry_directory(&name),
            )),
        }
    }

    /// Reads a layer directory: the §5.3 order of checks decides what a refusal is.
    fn read(&self, layer: &GitLayer<'_>, dir: &Dir) -> Step<(LayerFiles, Vec<SafeRelPath>)> {
        scan_verdict(Purpose::Build, dir.scan(Purpose::Build))
            .map(ReadLayer::into_parts)
            .map_err(|refusal| {
                refusal
                    .file_name_diagnostic(Self::source_text(layer))
                    .unwrap_or_else(|| cannot_store(Self::source_text(layer), self.cache_root()))
            })
    }

    fn layer(
        dir: Dir,
        entry: EntryName,
        files: LayerFiles,
        env_files: Vec<SafeRelPath>,
        sha: ObjectId,
        read: ReadHow,
    ) -> FetchedLayer {
        FetchedLayer {
            dir,
            entry,
            files,
            env_files,
            sha,
            rev: sha.short_rev(),
            read,
        }
    }

    /// A failed git step as `cannot fetch '<source>': <line>`.
    fn fetch_failure(layer: &GitLayer<'_>, failure: &GitFailure) -> Diagnostic {
        let line = match failure {
            GitFailure::Exited { stderr, .. } => FetchFailureLine::from_stderr(stderr),
            GitFailure::Spawn(_)
            | GitFailure::Io(_)
            | GitFailure::TimedOut { .. }
            | GitFailure::OutputTooLarge
            | GitFailure::Unreadable(_) => {
                FetchFailureLine::from_stderr(failure.to_string().as_bytes())
            }
        };
        cannot_fetch(Self::source_text(layer), line)
    }

    /// Steps 2 and 3.
    fn clone_and_publish(
        &self,
        layer: &GitLayer<'_>,
        locked_sha: Option<ObjectId>,
    ) -> Step<FetchedLayer> {
        let temp =
            TempEntry::create(self.cache).map_err(|error| self.cache_failure(layer, error))?;
        let clone = PathBuf::from(self.cache.real_path().as_str()).join(temp.name());
        let fail = |failure: GitFailure| Self::fetch_failure(layer, &failure);
        self.git
            .clone_source(
                layer.origin.transport(),
                OsStr::new(layer.origin.argument()),
                &clone,
            )
            .map_err(fail)?;
        let rev = locked_sha.map_or_else(|| layer.git_ref.clone(), |sha| GitRef::of_commit(&sha));
        let commit = self
            .git
            .rev_parse_commit(&clone, ReadScope::CacheEntry, &rev)
            .map_err(fail)?
            .ok_or_else(|| cannot_fetch(Self::source_text(layer), None))?;
        self.git.checkout(&clone, &commit).map_err(fail)?;
        let (files, env_files) = self.read(layer, temp.dir())?;
        let name = EntryName::Git {
            key: layer.origin.key(),
            commit,
        };
        match temp
            .publish(&name)
            .map_err(|error| self.cache_failure(layer, error))?
        {
            Published::Created(dir) => Ok(Self::layer(
                dir,
                name,
                files,
                env_files,
                commit,
                ReadHow::Fetched,
            )),
            Published::Existing(dir) => {
                let (existing, existing_env) = self.read(layer, &dir)?;
                if ContentHash::of(&existing) == ContentHash::of(&files) {
                    Ok(Self::layer(
                        dir,
                        name,
                        existing,
                        existing_env,
                        commit,
                        ReadHow::Fetched,
                    ))
                } else {
                    Err(cache_differs_from_clone(
                        Self::source_text(layer),
                        self.entry_directory(&name),
                    ))
                }
            }
        }
    }
}
