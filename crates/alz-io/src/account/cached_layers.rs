//! The layers of a repository's stack as account resolution asks for them (spec §7.13 "No fetch
//! on resolve"): read from the layer cache only, only when step 4 gets to them, and never for a
//! repository that is not allowed. It holds no `GitRunner`, [`RepoRoot`](crate::RepoRoot) or
//! [`StateDir`](super::StateDir), so it cannot fetch, run git or write.

use alz_core::account::Stack;
use alz_core::account::{AccountFolders, AllowedSource, CachedDeclaration, Miss, MissReason};
use alz_core::cache::CacheLocation;
use alz_core::path::AbsolutePath;
use alz_core::stack::Manifest;

use super::cached_reader::{CachedDeclarationReader, recorded_sources};
use super::home::Home;
use super::root_walk::{ManifestState, Repository};
use crate::EnvSnapshot;
use crate::fs::{CacheTrust, CacheView, ReadOnlyDir};

/// The stack of a repository whose manifest is read, or of nothing.
#[derive(Debug)]
pub struct CachedLayers<'a> {
    stack: Option<(&'a AbsolutePath, &'a Manifest)>,
    folders: &'a AccountFolders,
    home: &'a Home,
    env: &'a EnvSnapshot,
}

impl<'a> CachedLayers<'a> {
    /// The layers of `repository`. Resolution asks for them only of a root whose manifest is
    /// read; for any other repository the stack is empty.
    #[must_use]
    pub fn of(
        repository: &'a Repository,
        folders: &'a AccountFolders,
        home: &'a Home,
        env: &'a EnvSnapshot,
    ) -> Self {
        let stack = match repository {
            Repository::Root(root) => match root.manifest() {
                ManifestState::Read(manifest) => Some((root.path(), &**manifest)),
                ManifestState::Unreadable | ManifestState::Invalid => None,
            },
            Repository::NoRoot(_) => None,
        };
        Self {
            stack,
            folders,
            home,
            env,
        }
    }

    /// Runs `ask` on a reader of the stack, or says why none can be made.
    fn reading<T>(
        &self,
        ask: impl FnOnce(&CachedDeclarationReader<'_>, &Manifest) -> T,
    ) -> Result<T, MissReason> {
        let (root, manifest) = self.stack.ok_or(MissReason::Unreadable)?;
        let home = self.home.path().map(AbsolutePath::as_str);
        let xdg = self.env.xdg_cache_home().and_then(std::ffi::OsStr::to_str);
        let location =
            CacheLocation::from_env(home, xdg).map_err(|_none| MissReason::Unreadable)?;
        let default = CacheLocation::from_env(home, None).ok();
        let trust = CacheTrust::new(location, default, self.env.mise_or_direnv_active());
        let view = CacheView::open(&trust, self.folders, root)?;
        let repository = ReadOnlyDir::open(std::path::Path::new(root.as_str()))
            .map_err(|_unreadable| MissReason::Unreadable)?;
        let reader = CachedDeclarationReader::new(&repository, root, self.folders, &view)?;
        Ok(ask(&reader, manifest))
    }
}

impl Stack for CachedLayers<'_> {
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "`reading` hands the closure a reader of any lifetime, which the method path cannot name"
    )]
    fn declaration(&self) -> CachedDeclaration {
        self.reading(|reader, manifest| reader.read(manifest))
            .unwrap_or_else(|reason| CachedDeclaration::Miss(Miss::of(reason)))
    }

    fn contains(&self, recorded: &AllowedSource) -> bool {
        self.stack.is_some_and(|(root, manifest)| {
            recorded_sources(root, self.folders, manifest)
                .iter()
                .any(|source| source == recorded.as_str())
        })
    }
}
