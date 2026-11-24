//! Locating, vetting and reading `accounts.yaml` and `allowed.yaml` (spec §7.13 "Both files").
//!
//! The checks run in the spec's order and the first that fails ignores the file with one warning;
//! nothing further is opened. The file is read at its real path through no link, at most 1 MiB,
//! and its owner and mode are those of the handle that was read.

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use alz_core::account::{AccountFolders, AllowFile, Reading, UserConfig, lies_within};
use alz_core::path::AbsolutePath;
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped, Where};
use rustix::io::Errno;

use super::home::Home;
use super::ignore::{DirectoryProblem, IgnoreReason};
use super::resolved::{ResolvedAllowState, ResolvedUserConfig};
use crate::EnvSnapshot;
use crate::fs::{ReadError, ReadOnlyDir, RepositoryPaths};
use crate::resolve::folders::{account_folders, account_folders_at};

/// The most a file may hold (spec §7.13 "Both files").
const CAP: usize = 1_048_576;

/// Which of the two files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    UserConfig,
    AllowState,
}

impl Which {
    const fn name(self) -> &'static str {
        match self {
            Self::UserConfig => "accounts.yaml",
            Self::AllowState => "allowed.yaml",
        }
    }

    const fn code(self) -> Code {
        match self {
            Self::UserConfig => Code::AccountConfig,
            Self::AllowState => Code::AccountAllowed,
        }
    }

    const fn variable(self) -> &'static str {
        match self {
            Self::UserConfig => "XDG_CONFIG_HOME",
            Self::AllowState => "XDG_STATE_HOME",
        }
    }

    const fn fallback(self) -> &'static [&'static str] {
        match self {
            Self::UserConfig => &[".config"],
            Self::AllowState => &[".local", "state"],
        }
    }
}

/// What became of one account file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// The file passed every check and was read. Its content may still be ignored in part, each
    /// part with a warning.
    Read,
    /// There is no such file: not a problem, the step is skipped.
    NotFound,
    /// A check failed; the warning names the reason.
    Ignored(IgnoreReason),
}

/// One account file as it was read: where it is, what became of it, what it holds, and the
/// warnings. A file that was not read holds the default (empty) value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded<T> {
    path: Option<AbsolutePath>,
    status: FileStatus,
    value: T,
    warnings: Diagnostics,
}

impl<T> Loaded<T> {
    /// The path the file is looked for at, before any link is followed. `None` when `$HOME` is
    /// not an absolute path.
    #[must_use]
    pub const fn path(&self) -> Option<&AbsolutePath> {
        self.path.as_ref()
    }

    /// What became of the file.
    #[must_use]
    pub const fn status(&self) -> &FileStatus {
        &self.status
    }

    /// What the file holds.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The warnings, in the order they were found.
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
    }

    /// What the file holds, taken.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

/// Why a file was not read, before any content.
enum Stop {
    Missing,
    Ignored(IgnoreReason),
}

/// A file that passed every check and was read.
struct Trusted {
    bytes: Vec<u8>,
    home: AbsolutePath,
    /// The `.git` that makes `accounts.yaml` part of a work tree: a warning, not a refusal.
    work_tree: Option<AbsolutePath>,
}

/// The two account files of one user, one environment and, when there is one, one repository.
///
/// It reads and never writes: `allowed.yaml` is only written by `alz account allow`.
#[derive(Debug, Clone)]
pub struct AccountFiles {
    home: Home,
    env: EnvSnapshot,
    repository: Option<RepositoryPaths>,
    folders: AccountFolders,
    owner: Option<u32>,
}

impl AccountFiles {
    /// The files of the user who runs this process (the effective user id owns what is trusted).
    /// A `home` that is not absolute ignores both files and sets no account folder.
    #[must_use]
    pub fn new(home: Home, env: &EnvSnapshot, repository: Option<RepositoryPaths>) -> Self {
        let folders = home.path().map_or_else(
            || account_folders(env),
            |path| account_folders_at(path, env),
        );
        Self {
            home,
            env: env.clone(),
            repository,
            folders,
            owner: Some(rustix::process::geteuid().as_raw()),
        }
    }

    /// The same files judged for another owner: `None` for a platform where the user id cannot
    /// be read, which ignores both files.
    #[must_use]
    pub fn with_owner(self, owner: Option<u32>) -> Self {
        Self { owner, ..self }
    }

    /// The warning for a `$HOME` that is not an absolute path, once; `None` otherwise. Each file
    /// is then [`ignored`](FileStatus::Ignored) without a warning of its own.
    #[must_use]
    pub fn home_warning(&self) -> Option<Diagnostic> {
        match self.home {
            Home::NotAbsolute => Some(Diagnostic::warning(
                Code::AccountConfig,
                None,
                Message::new(
                    "HOME is not an absolute path; accounts.yaml and allowed.yaml are ignored \
                     and no account folder is set",
                ),
            )),
            Home::Absolute(_) => None,
        }
    }

    /// The account folders this machine and environment have, for the callers that compare paths
    /// with them.
    #[must_use]
    pub const fn folders(&self) -> &AccountFolders {
        &self.folders
    }

    /// `accounts.yaml`, read and checked again once its paths are real.
    #[must_use]
    pub fn user_config(&self) -> Loaded<ResolvedUserConfig> {
        self.load(Which::UserConfig, |bytes, file, _home| {
            ResolvedUserConfig::resolve(
                UserConfig::from_bytes(bytes, file),
                &self.home_or_root(),
                &self.folders,
                file,
            )
        })
    }

    /// `allowed.yaml`, read and checked again once its roots are real.
    #[must_use]
    pub fn allow_state(&self) -> Loaded<ResolvedAllowState> {
        self.load(Which::AllowState, |bytes, file, _home| {
            ResolvedAllowState::resolve(AllowFile::from_bytes(bytes, file), &self.folders, file)
        })
    }

    fn home_or_root(&self) -> AbsolutePath {
        self.home.path().cloned().unwrap_or_else(AbsolutePath::root)
    }

    fn load<T: Default>(
        &self,
        which: Which,
        parse: impl FnOnce(&[u8], &Where, &AbsolutePath) -> Reading<T>,
    ) -> Loaded<T> {
        let fetched = self.fetch(which);
        let file = Where::given(
            fetched
                .shown
                .clone()
                .unwrap_or_else(|| Stripped::from_untrusted(which.name())),
        );
        let (status, reading) = match fetched.outcome {
            Ok(trusted) => (
                FileStatus::Read,
                parse(&trusted.bytes, &file, &trusted.home),
            ),
            Err(Stop::Missing) => (FileStatus::NotFound, Reading::new(T::default())),
            Err(Stop::Ignored(reason)) => (FileStatus::Ignored(reason), Reading::new(T::default())),
        };
        let (value, read_warnings) = reading.into_parts();
        Loaded {
            path: fetched.path,
            status,
            value,
            warnings: fetched.warnings.merge(read_warnings),
        }
    }

    fn xdg(&self, which: Which) -> Option<&std::ffi::OsStr> {
        match which {
            Which::UserConfig => self.env.xdg_config_home(),
            Which::AllowState => self.env.xdg_state_home(),
        }
    }

    fn fetch(&self, which: Which) -> Fetched {
        let Home::Absolute(home) = &self.home else {
            return Fetched {
                path: None,
                shown: None,
                outcome: Err(Stop::Ignored(IgnoreReason::HomeNotAbsolute)),
                warnings: Diagnostics::new(),
            };
        };
        let base = xdg_base(self.xdg(which), home, which.fallback());
        let path = joined(&base.dir, &["agent-layerz", which.name()]);
        let real_home = settled(home);
        let shown = display(&path, &[home, &real_home]);
        let outcome = self.vet(which, home, &real_home, &base, &path);
        let relative = base.relative.then(|| {
            Diagnostic::warning(
                which.code(),
                None,
                Message::new(which.variable())
                    .then(" is not an absolute path; ")
                    .then(which.name())
                    .then(" is looked for in the default place"),
            )
        });
        let problem = match &outcome {
            Err(Stop::Ignored(reason)) => Some(Diagnostic::warning(
                which.code(),
                None,
                reason.after(
                    Message::new(which.name())
                        .then(" (")
                        .then(shown.clone())
                        .then(") is ignored: "),
                ),
            )),
            Ok(_) | Err(Stop::Missing) => None,
        };
        let in_work_tree = match &outcome {
            Ok(Trusted {
                work_tree: Some(marker),
                ..
            }) => Some(Diagnostic::warning(
                which.code(),
                None,
                Message::new(which.name())
                    .then(" (")
                    .then(shown.clone())
                    .then(") lies inside a git work tree (")
                    .then(display(marker, &[home, &real_home]))
                    .then("); keep it outside every repository"),
            )),
            Ok(_) | Err(_) => None,
        };
        Fetched {
            path: Some(path),
            shown: Some(shown),
            outcome,
            warnings: [relative, problem, in_work_tree]
                .into_iter()
                .flatten()
                .fold(Diagnostics::new(), Diagnostics::with),
        }
    }

    /// The checks of "Both files", in order, and the read.
    fn vet(
        &self,
        which: Which,
        home: &AbsolutePath,
        real_home: &AbsolutePath,
        base: &XdgBase,
        path: &AbsolutePath,
    ) -> Result<Trusted, Stop> {
        // Nothing below calls the file system until the text of the path has passed.
        let owner = self.owner.ok_or(Stop::Ignored(IgnoreReason::NoUserId))?;
        refuse_if(self.folders.touches(path), || {
            IgnoreReason::NamesAccountFolder
        })?;
        self.refuse_moved(which, home, base)?;
        let real = canonical(path).map_err(Stop::from)?;
        refuse_if(self.folders.touches(&real), || {
            IgnoreReason::InsideAccountFolder
        })?;
        refuse_if(self.inside_repository(&real), || {
            IgnoreReason::InsideRepository
        })?;
        let marker = work_tree_marker(&real).or_else(|| work_tree_marker(path));
        let work_tree = match (which, marker) {
            (Which::AllowState, Some(marker)) => {
                return Err(Stop::Ignored(IgnoreReason::InsideWorkTree(display(
                    &marker,
                    &[home, real_home],
                ))));
            }
            (Which::UserConfig, marker) => marker,
            (Which::AllowState, None) => None,
        };
        let file = ReadOnlyDir::open(Path::new("/"))
            .and_then(|root| root.read_absolute(&real, CAP))
            .map_err(|error| match error {
                ReadError::Missing => Stop::Missing,
                ReadError::NotRegular | ReadError::TooLarge | ReadError::Io(_) => {
                    Stop::Ignored(IgnoreReason::Unreadable)
                }
            })?;
        refuse_if(file.owner() != owner, || IgnoreReason::NotOwned)?;
        refuse_if(file.writable_by_group_or_others(), || {
            IgnoreReason::GroupOrOthersCanWrite
        })?;
        let located_dir = parent(path);
        let directories: BTreeSet<AbsolutePath> =
            [parent(&real), settled(&located_dir)].into_iter().collect();
        match directories
            .iter()
            .find_map(|directory| weak_directory(directory, home, real_home, owner))
        {
            Some(reason) => Err(Stop::Ignored(reason)),
            None => Ok(Trusted {
                bytes: file.into_bytes(),
                home: home.clone(),
                work_tree,
            }),
        }
    }

    /// An `XDG_*` variable that moved the lookup while mise or direnv is active is refused before
    /// the existence check, so a folder that does not exist yet is still reported (spec §7.13).
    fn refuse_moved(&self, which: Which, home: &AbsolutePath, base: &XdgBase) -> Result<(), Stop> {
        let moved = self.env.mise_or_direnv_active()
            && base.from_variable
            && settled(&base.dir) != settled(&joined(home, which.fallback()));
        refuse_if(moved, || IgnoreReason::XdgMoved(which.variable()))
    }

    fn inside_repository(&self, real: &AbsolutePath) -> bool {
        self.repository.as_ref().is_some_and(|repository| {
            lies_within(real, repository.real()) || lies_within(real, repository.as_written())
        })
    }
}

/// What looking for one file found, with the warnings that raised.
struct Fetched {
    path: Option<AbsolutePath>,
    shown: Option<Stripped>,
    outcome: Result<Trusted, Stop>,
    warnings: Diagnostics,
}

fn refuse_if(condition: bool, reason: impl FnOnce() -> IgnoreReason) -> Result<(), Stop> {
    if condition {
        Err(Stop::Ignored(reason()))
    } else {
        Ok(())
    }
}

/// Where an `XDG_*` base is, and how the variable counted: an unset or empty one is unset, a
/// relative one is unset with a warning (the XDG Base Directory specification).
struct XdgBase {
    dir: AbsolutePath,
    from_variable: bool,
    relative: bool,
}

fn xdg_base(value: Option<&std::ffi::OsStr>, home: &AbsolutePath, fallback: &[&str]) -> XdgBase {
    let default = |relative| XdgBase {
        dir: joined(home, fallback),
        from_variable: false,
        relative,
    };
    value.map_or_else(
        || default(false),
        |value| match value.to_str() {
            Some("") => default(false),
            Some(text) => AbsolutePath::try_from(text).map_or_else(
                |_relative| default(true),
                |dir| XdgBase {
                    dir,
                    from_variable: true,
                    relative: false,
                },
            ),
            None => default(true),
        },
    )
}

/// `base` with `parts` below it. `base` is absolute and `parts` hold no NUL, so the text is an
/// absolute path; were it not, `base` itself is the answer, which a caller then refuses.
fn joined(base: &AbsolutePath, parts: &[&str]) -> AbsolutePath {
    let text = std::iter::once(base.as_str().trim_end_matches('/'))
        .chain(parts.iter().copied())
        .collect::<Vec<_>>()
        .join("/");
    AbsolutePath::try_from(text.as_str()).unwrap_or_else(|_not_absolute| base.clone())
}

/// The folder that holds `path`; `/` holds itself.
fn parent(path: &AbsolutePath) -> AbsolutePath {
    let names: Vec<&str> = path.components().collect();
    prefix(&names, names.len().saturating_sub(1))
}

/// The first `count` names of `names` as an absolute path.
fn prefix(names: &[&str], count: usize) -> AbsolutePath {
    let text = format!(
        "/{}",
        names
            .iter()
            .take(count)
            .copied()
            .collect::<Vec<_>>()
            .join("/")
    );
    AbsolutePath::try_from(text.as_str()).unwrap_or_else(|_not_absolute| AbsolutePath::root())
}

/// Why a path has no real path.
enum Lookup {
    Missing,
    Unresolvable,
}

impl From<Lookup> for Stop {
    fn from(lookup: Lookup) -> Self {
        match lookup {
            Lookup::Missing => Self::Missing,
            Lookup::Unresolvable => Self::Ignored(IgnoreReason::PathUnresolvable),
        }
    }
}

/// The real path of `path`: only the names on the way are looked at.
fn canonical(path: &AbsolutePath) -> Result<AbsolutePath, Lookup> {
    std::fs::canonicalize(path.as_str())
        .map_err(
            |error| match error.raw_os_error().map(Errno::from_raw_os_error) {
                Some(Errno::NOENT | Errno::NOTDIR) => Lookup::Missing,
                Some(_) | None => Lookup::Unresolvable,
            },
        )
        .and_then(|real| {
            real.to_str()
                .and_then(|text| AbsolutePath::try_from(text).ok())
                .ok_or(Lookup::Unresolvable)
        })
}

/// The real path, or the path itself when there is none.
fn settled(path: &AbsolutePath) -> AbsolutePath {
    canonical(path).unwrap_or_else(|_missing| path.clone())
}

/// The path with `~` for the home folder, stripped of control characters, for a warning.
fn display(path: &AbsolutePath, homes: &[&AbsolutePath]) -> Stripped {
    let text = path.as_str();
    homes
        .iter()
        .find_map(|home| {
            let base = home.as_str().trim_end_matches('/');
            text.strip_prefix(base)
                .filter(|rest| !base.is_empty() && (rest.is_empty() || rest.starts_with('/')))
                .map(|rest| format!("~{rest}"))
        })
        .map_or_else(
            || Stripped::from_untrusted(text),
            |shown| Stripped::from_untrusted(&shown),
        )
}

/// The `.git` (a folder or a file) that makes `path` part of a git work tree: only `lstat` of
/// `.git` in each folder above the path, as written; no git runs (spec §7.13 "Both files").
fn work_tree_marker(path: &AbsolutePath) -> Option<AbsolutePath> {
    let names: Vec<&str> = path.components().collect();
    (0..names.len())
        .rev()
        .map(|count| joined(&prefix(&names, count), &[".git"]))
        .find(|marker| std::fs::symlink_metadata(marker.as_str()).is_ok())
}

/// The first directory that could let another user swap the file: `directory` and each parent
/// strictly below `$HOME`, or only `directory` when it lies outside `$HOME`. A directory is weak
/// when group or others can write it, or the user does not own it (root is accepted outside
/// `$HOME`).
fn weak_directory(
    directory: &AbsolutePath,
    home: &AbsolutePath,
    real_home: &AbsolutePath,
    owner: u32,
) -> Option<IgnoreReason> {
    let below = directory != real_home && lies_within(directory, real_home);
    let names: Vec<&str> = directory.components().collect();
    let home_depth = real_home.components().count();
    let chain: Vec<AbsolutePath> = if below {
        (home_depth.saturating_add(1)..=names.len())
            .rev()
            .map(|count| prefix(&names, count))
            .collect()
    } else {
        vec![directory.clone()]
    };
    chain.iter().find_map(|folder| {
        problem_with(folder, owner, below).map(|problem| IgnoreReason::Directory {
            problem,
            directory: display(folder, &[home, real_home]),
        })
    })
}

fn problem_with(directory: &AbsolutePath, owner: u32, below: bool) -> Option<DirectoryProblem> {
    match std::fs::symlink_metadata(directory.as_str()) {
        Err(_) => Some(DirectoryProblem::CannotBeChecked),
        Ok(status) if status.mode() & 0o022 != 0 => Some(DirectoryProblem::GroupOrOthersCanWrite),
        Ok(status) if status.uid() == owner || (!below && status.uid() == 0) => None,
        Ok(_) => Some(DirectoryProblem::NotOwned),
    }
}

#[cfg(test)]
mod tests {
    use alz_core::path::AbsolutePath;
    use alz_test_support::ScratchDir;

    use super::{DirectoryProblem, IgnoreReason, weak_directory};

    #[test]
    fn a_directory_owned_by_another_user_is_weak_below_home() {
        let scratch = ScratchDir::new().expect("a scratch directory");
        scratch.make_dirs("home/a/b").expect("folders");
        let real = scratch.real_path().expect("real path");
        let at = |relative: &str| {
            AbsolutePath::try_from(real.join(relative).to_str().expect("text")).expect("absolute")
        };
        let owner = rustix::process::geteuid().as_raw();
        let reason = weak_directory(
            &at("home/a/b"),
            &at("home"),
            &at("home"),
            owner.wrapping_add(1),
        );
        assert!(
            matches!(
                reason,
                Some(IgnoreReason::Directory {
                    problem: DirectoryProblem::NotOwned,
                    ..
                })
            ),
            "{reason:?}"
        );
        assert_eq!(
            weak_directory(&at("home/a/b"), &at("home"), &at("home"), owner),
            None
        );
    }
}
