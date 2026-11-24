//! Where `allowed.yaml` is written, decided the way the reader decides what it reads (spec §7.13
//! "Both files"). Before any file system call: `$HOME` absolute, a user id, a path that names no
//! account folder and no `XDG_STATE_HOME` that mise or direnv moved. Then a regular file, or a link
//! to one, is kept and written at its real path; a missing file gets the folder it is looked for
//! in, created with mode `0700` only after that place is known to lie outside the repository, every
//! account folder and every git work tree. A dangling link and anything that is not a regular file
//! refuse. Nothing is repaired.

use std::io;
use std::os::fd::OwnedFd;

use alz_core::account::lies_within;
use alz_core::path::AbsolutePath;
use rustix::fs::{self, Mode, OFlags};
use rustix::io::Errno;

use super::files::{
    AccountFiles, FileStatus, Which, canonical, display, joined, parent, settled, weak_directory,
    work_tree_marker, xdg_base,
};
use super::home::Home;
use super::ignore::IgnoreReason;
use super::state_error::StateError;

/// Where the file is, as real paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Place {
    /// The real folder that holds the file, the lock file and the temporary file.
    pub(super) dir: AbsolutePath,
    /// The name of the real file (not `allowed.yaml` for a link to another name).
    pub(super) name: String,
    /// The file exists.
    pub(super) existing: bool,
    /// The file or a folder above it is a link: its real path differs from where it is looked for.
    pub(super) linked: bool,
}

const WHICH: Which = Which::AllowState;

fn refuse(condition: bool, reason: impl FnOnce() -> IgnoreReason) -> Result<(), StateError> {
    if condition {
        Err(StateError::Place(reason()))
    } else {
        Ok(())
    }
}

/// Decides the place, and with `create` makes the missing folders.
pub(super) fn place(files: &AccountFiles, create: bool) -> Result<Place, StateError> {
    let Home::Absolute(home) = &files.home else {
        return Err(StateError::Place(IgnoreReason::HomeNotAbsolute));
    };
    let owner = files
        .owner
        .ok_or(StateError::Place(IgnoreReason::NoUserId))?;
    let base = xdg_base(files.env.xdg_state_home(), home, WHICH.fallback());
    let located = joined(&base.dir, &["agent-layerz", WHICH.name()]);
    refuse(files.folders.touches(&located), || {
        IgnoreReason::NamesAccountFolder
    })?;
    let real_home = settled(home);
    let moved = files.env.mise_or_direnv_active()
        && base.from_variable
        && settled(&base.dir) != settled(&joined(home, WHICH.fallback()));
    refuse(moved, || IgnoreReason::XdgMoved(WHICH.variable()))?;
    let context = Context {
        files,
        home,
        real_home: &real_home,
        located: &located,
        owner,
    };
    match std::fs::symlink_metadata(located.as_str()) {
        Ok(_) => context.existing(),
        Err(error) if is_absent(&error) => context.missing(create),
        Err(_) => Err(StateError::Place(IgnoreReason::PathUnresolvable)),
    }
}

fn is_absent(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error().map(Errno::from_raw_os_error),
        Some(Errno::NOENT | Errno::NOTDIR)
    )
}

struct Context<'a> {
    files: &'a AccountFiles,
    home: &'a AbsolutePath,
    real_home: &'a AbsolutePath,
    located: &'a AbsolutePath,
    owner: u32,
}

impl Context<'_> {
    /// The file is there: the reader's checks decide, then the place is its real path.
    fn existing(&self) -> Result<Place, StateError> {
        let loaded = self.files.allow_state();
        match loaded.status() {
            FileStatus::Ignored(reason) => return Err(StateError::Place(reason.clone())),
            FileStatus::NotFound => return Err(StateError::DanglingLink),
            FileStatus::Read => {}
        }
        let real = canonical(self.located).map_err(|_unresolved| StateError::DanglingLink)?;
        let name = real
            .components()
            .last()
            .ok_or(StateError::Place(IgnoreReason::Unreadable))?
            .to_owned();
        Ok(Place {
            dir: parent(&real),
            name,
            existing: true,
            linked: real != lexical_real(self.located, self.home, self.real_home),
        })
    }

    /// The checks of the reader on a place whose file is not there (yet).
    fn real_place_problem(&self, file: &AbsolutePath) -> Result<(), StateError> {
        refuse(self.files.folders.touches(file), || {
            IgnoreReason::InsideAccountFolder
        })?;
        refuse(self.files.inside_repository(file), || {
            IgnoreReason::InsideRepository
        })?;
        match work_tree_marker(file).or_else(|| work_tree_marker(self.located)) {
            Some(marker) => Err(StateError::Place(IgnoreReason::InsideWorkTree(display(
                &marker,
                &[self.home, self.real_home],
            )))),
            None => Ok(()),
        }
    }

    fn weak(&self, directory: &AbsolutePath) -> Result<(), StateError> {
        weak_directory(directory, self.home, self.real_home, self.owner)
            .map_or(Ok(()), |reason| Err(StateError::Place(reason)))
    }

    /// The file is missing: its folder is found from the nearest one that exists.
    fn missing(&self, create: bool) -> Result<Place, StateError> {
        let (ancestor, rest) = nearest_existing(&parent(self.located))?;
        let real_ancestor =
            canonical(&ancestor).map_err(|_unresolved| StateError::FolderUnusable)?;
        if !std::fs::symlink_metadata(real_ancestor.as_str()).is_ok_and(|status| status.is_dir()) {
            return Err(StateError::FolderUnusable);
        }
        let names: Vec<&str> = rest.iter().map(String::as_str).collect();
        let would_be = joined(&real_ancestor, &names);
        self.real_place_problem(&joined(&would_be, &[WHICH.name()]))?;
        self.weak(&real_ancestor)?;
        let place_in = |dir: AbsolutePath| Place {
            dir,
            name: WHICH.name().to_owned(),
            existing: false,
            linked: false,
        };
        if rest.is_empty() || !create {
            return Ok(place_in(would_be));
        }
        make_folders(&real_ancestor, &rest).map_err(|_failed| {
            StateError::FolderNotCreated(display(&would_be, &[self.real_home]))
        })?;
        let real_dir = canonical(&would_be).map_err(|_unresolved| StateError::FolderUnusable)?;
        self.real_place_problem(&joined(&real_dir, &[WHICH.name()]))?;
        self.weak(&real_dir)?;
        Ok(place_in(real_dir))
    }
}

/// The nearest folder at or above `start` that exists, and the names below it.
fn nearest_existing(start: &AbsolutePath) -> Result<(AbsolutePath, Vec<String>), StateError> {
    let names: Vec<&str> = start.components().collect();
    (0..=names.len())
        .rev()
        .map(|count| {
            let kept: Vec<&str> = names.iter().take(count).copied().collect();
            (joined(&AbsolutePath::root(), &kept), count)
        })
        .find(|(existing, _count)| std::fs::symlink_metadata(existing.as_str()).is_ok())
        .map(|(existing, count)| {
            let rest = names
                .iter()
                .skip(count)
                .map(|name| (*name).to_owned())
                .collect();
            (existing, rest)
        })
        .ok_or(StateError::FolderUnusable)
}

/// Each missing folder below `real_ancestor`, made with `mkdirat` and mode `0700` and opened with
/// `O_NOFOLLOW`, so a link that appeared is never followed.
fn make_folders(real_ancestor: &AbsolutePath, rest: &[String]) -> io::Result<()> {
    let start = fs::open(
        real_ancestor.as_str(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    rest.iter()
        .try_fold(start, |directory: OwnedFd, name| {
            match fs::mkdirat(&directory, name.as_str(), Mode::RWXU) {
                Ok(()) | Err(Errno::EXIST) => {}
                Err(errno) => return Err(io::Error::from(errno)),
            }
            fs::openat(
                &directory,
                name.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io::Error::from)
        })
        .map(drop)
}

/// Where `path` is when nothing on the way is a link: the path with `$HOME` spelled as its real
/// path (a temporary or network folder can be reached through a link of the system's own). A file
/// whose real path differs is a link, or lies below one.
fn lexical_real(
    path: &AbsolutePath,
    home: &AbsolutePath,
    real_home: &AbsolutePath,
) -> AbsolutePath {
    let base = home.as_str().trim_end_matches('/');
    path.as_str()
        .strip_prefix(base)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
        .filter(|_inside| lies_within(path, home))
        .and_then(|rest| {
            AbsolutePath::try_from(
                format!("{}{rest}", real_home.as_str().trim_end_matches('/')).as_str(),
            )
            .ok()
        })
        .unwrap_or_else(|| path.clone())
}
