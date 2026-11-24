//! [`StateDir`]: the only writer outside the repository besides the layer cache (spec §7.13 "Both
//! files", decision-20). It records or removes one repository's entry in `allowed.yaml`.
//!
//! Every path comes from `$HOME`, `XDG_STATE_HOME` and fixed names, never from repository text.
//! The place is decided as the reader decides what it reads ([`state_place`](super::state_place));
//! under an exclusive lock file next to the file that is replaced, the state is read again, the new
//! text is written to a temporary file in the same folder and renamed over the file. The lock file,
//! the temporary file and the rename are all relative to one folder handle, so nothing on the way
//! can be swapped for a link between the steps. Nothing is repaired: no `chmod` of an existing file
//! or folder, no dropped entry, no removed stale lock.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use alz_core::account::{AllowEntry, AllowFile, AllowState};
use alz_core::path::AbsolutePath;
use alz_core::{Stripped, Where};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, Stat};
use rustix::io::Errno;

use super::files::{AccountFiles, FileStatus, Loaded, display, joined, settled};
use super::resolved::ResolvedAllowState;
use super::state_error::{StateError, StateProblem};
use super::state_place::{Place, place};

/// The most `allowed.yaml` may hold (spec §7.13 "Both files").
const CAP: usize = 1_048_576;
/// How often a lock file that exists is looked at again.
const POLL: Duration = Duration::from_millis(50);
/// How long a lock file that exists is waited for.
const DEFAULT_WAIT: Duration = Duration::from_secs(10);

/// What one run of `alz account allow` changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowChange {
    /// Record the entry, replacing the one with the same root.
    Set(AllowEntry),
    /// Remove the entry of this repository root.
    Remove(AbsolutePath),
}

impl AllowChange {
    const fn root(&self) -> &AbsolutePath {
        match self {
            Self::Set(entry) => entry.root(),
            Self::Remove(root) => root,
        }
    }
}

/// What a change did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updated {
    /// The file was replaced.
    Written,
    /// The file already says it, or holds nothing to remove: nothing was written, and a remove with
    /// no file creates nothing.
    Unchanged,
}

/// A step the writer has reached, for a test or a progress line. Observing changes nothing.
#[derive(Debug)]
pub enum Stage<'a> {
    /// The lock file exists and is waited for; sent once.
    LockBusy,
    /// This run holds the lock file.
    LockHeld,
    /// The temporary file is written and flushed; the rename has not happened.
    TempWritten(&'a std::path::Path),
}

/// The state folder of this user: `${XDG_STATE_HOME:-~/.local/state}/agent-layerz`, as far as it
/// is safe to write there.
#[derive(Debug, Clone)]
pub struct StateDir {
    files: AccountFiles,
    wait: Duration,
}

impl StateDir {
    /// The writer for the files `files` reads: the same place, the same checks. It waits 10 seconds
    /// for a lock file that exists.
    #[must_use]
    pub const fn new(files: AccountFiles) -> Self {
        Self {
            files,
            wait: DEFAULT_WAIT,
        }
    }

    /// The same writer waiting `wait` for a lock file that exists.
    #[must_use]
    pub fn with_lock_wait(self, wait: Duration) -> Self {
        Self { wait, ..self }
    }

    /// The checks of a change made before anything is created or opened for writing: the place and
    /// the state of the file, without creating a folder, a lock file or a temporary file. `alz
    /// account allow` runs it before it asks anything.
    ///
    /// # Errors
    ///
    /// The first state of the place or of the file that the writer refuses.
    pub fn check(&self) -> Result<(), StateError> {
        let place = place(&self.files, false)?;
        if place.linked {
            linked_identity(&place)?;
        }
        problem_of(&self.files.allow_state()).map(drop)
    }

    /// Records or removes the entry for the repository root of `change`.
    ///
    /// # Errors
    ///
    /// A refusal ([`StateError`]), after which nothing was changed, except
    /// [`WrittenButUnreadable`](StateError::WrittenButUnreadable).
    pub fn update(&self, change: &AllowChange) -> Result<Updated, StateError> {
        self.update_observed(change, &|_stage| {})
    }

    /// As [`update`](Self::update), telling `observe` each [`Stage`] on the way.
    ///
    /// # Errors
    ///
    /// As [`update`](Self::update).
    pub fn update_observed(
        &self,
        change: &AllowChange,
        observe: &dyn Fn(Stage<'_>),
    ) -> Result<Updated, StateError> {
        // A change too large by itself is refused before a folder is made for it.
        if let AllowChange::Set(entry) = change
            && AllowState::from_entries(vec![entry.clone()])
                .to_yaml()
                .len()
                > CAP
        {
            return Err(StateError::TooLarge);
        }
        let first = place(&self.files, matches!(change, AllowChange::Set(_)))?;
        if matches!(change, AllowChange::Remove(_)) && !first.existing {
            return Ok(Updated::Unchanged);
        }
        let dir = open_dir(&first.dir)?;
        if first.linked {
            linked_identity(&first)?;
        }
        let lock = Lock::acquire(&dir, &first, &self.files, self.wait, observe)?;
        observe(Stage::LockHeld);
        let outcome = self.under_lock(change, &first, &dir, observe);
        drop(lock);
        outcome
    }

    fn under_lock(
        &self,
        change: &AllowChange,
        first: &Place,
        dir: &OwnedFd,
        observe: &dyn Fn(Stage<'_>),
    ) -> Result<Updated, StateError> {
        // Read again, with the reader's own checks, now that nobody else is writing.
        let place = place(&self.files, false)?;
        if place.dir != first.dir || place.name != first.name {
            return Err(StateError::Changed);
        }
        if first.existing && !place.existing {
            return Err(StateError::Changed);
        }
        let expected = if place.linked {
            Some(linked_identity(&place)?)
        } else {
            None
        };
        let loaded = self.files.allow_state();
        let state = problem_of(&loaded)?;
        let root = change.root();
        let current = state.entries().iter().find(|entry| entry.root() == root);
        let others = state
            .entries()
            .iter()
            .filter(|entry| entry.root() != root)
            .chain(state.unresolved())
            .cloned();
        let wanted = match change {
            AllowChange::Set(entry) => Some(entry),
            AllowChange::Remove(_) => None,
        };
        if current == wanted {
            return Ok(Updated::Unchanged);
        }
        let next = AllowState::from_entries(others.chain(wanted.cloned()).collect());
        let text = next.to_yaml();
        if text.len() > CAP {
            return Err(StateError::TooLarge);
        }
        if !reads_back(&text, &next) {
            return Err(StateError::WouldNotReadBack);
        }
        write_through_temp(dir, &place, &text, expected.as_ref(), observe)?;
        self.read_back(root, wanted)
    }

    fn read_back(
        &self,
        root: &AbsolutePath,
        wanted: Option<&AllowEntry>,
    ) -> Result<Updated, StateError> {
        let loaded = self.files.allow_state();
        let reads = matches!(loaded.status(), FileStatus::Read)
            && loaded.value().is_list()
            && loaded.value().ignored().is_empty()
            && loaded
                .value()
                .entries()
                .iter()
                .find(|entry| entry.root() == root)
                == wanted;
        if reads {
            Ok(Updated::Written)
        } else {
            Err(StateError::WrittenButUnreadable)
        }
    }
}

/// The state the reader found, or why alz refuses to change it.
fn problem_of(loaded: &Loaded<ResolvedAllowState>) -> Result<&ResolvedAllowState, StateError> {
    match loaded.status() {
        FileStatus::Ignored(reason) => {
            Err(StateError::State(StateProblem::Ignored(reason.clone())))
        }
        FileStatus::Read if !loaded.value().is_list() => {
            Err(StateError::State(StateProblem::NotAList))
        }
        FileStatus::Read if !loaded.value().ignored().is_empty() => Err(StateError::State(
            StateProblem::IgnoredEntries(loaded.value().ignored().to_vec()),
        )),
        FileStatus::NotFound | FileStatus::Read => Ok(loaded.value()),
    }
}

/// Whether `text` reads as exactly `state`, with nothing ignored.
fn reads_back(text: &str, state: &AllowState) -> bool {
    let file = Where::given(Stripped::from_untrusted("allowed.yaml"));
    match AllowFile::from_bytes(text.as_bytes(), &file).into_parts().0 {
        AllowFile::List {
            state: read,
            ignored,
            ..
        } => ignored.is_empty() && &read == state,
        AllowFile::Ignored => false,
    }
}

fn open_dir(dir: &AbsolutePath) -> Result<OwnedFd, StateError> {
    fs::open(
        dir.as_str(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(StateError::from)
}

/// The file a link leads to, when alz may replace it: empty, `[]`, or starting with the header alz
/// writes. Anything else may be somebody else's file. The result tells which file was read, so a
/// swap before the rename is noticed.
fn linked_identity(place: &Place) -> Result<Stat, StateError> {
    let dir = open_dir(&place.dir)?;
    let file = fs::openat(
        &dir,
        place.name.as_str(),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_unreadable| StateError::LinkedElsewhere)?;
    let status = fs::fstat(&file)?;
    if FileType::from_raw_mode(status.st_mode) != FileType::RegularFile {
        return Err(StateError::LinkedElsewhere);
    }
    let mut bytes = Vec::new();
    std::fs::File::from(file)
        .take(u64::try_from(CAP).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)?;
    let text = std::str::from_utf8(&bytes).map_err(|_not_text| StateError::LinkedElsewhere)?;
    let header = AllowState::default().to_yaml();
    let first_line = header.split_inclusive('\n').next().unwrap_or_default();
    let trimmed = text.trim();
    if bytes.len() <= CAP && (trimmed.is_empty() || trimmed == "[]" || text.starts_with(first_line))
    {
        Ok(status)
    } else {
        Err(StateError::LinkedElsewhere)
    }
}

fn same_file(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev && left.st_ino == right.st_ino
}

/// The lock file, held. It is removed on drop, and only when it is still the file this run made.
struct Lock<'a> {
    dir: &'a OwnedFd,
    name: String,
    file: OwnedFd,
}

impl<'a> Lock<'a> {
    /// Creates `<name>.lock` with `O_CREAT|O_EXCL|O_NOFOLLOW` and mode `0600`; one that exists, a
    /// link included, is polled every 50 ms until `wait` is over.
    fn acquire(
        dir: &'a OwnedFd,
        place: &Place,
        files: &AccountFiles,
        wait: Duration,
        observe: &dyn Fn(Stage<'_>),
    ) -> Result<Self, StateError> {
        let name = format!("{}.lock", place.name);
        let shown = || {
            let home = files
                .home
                .path()
                .cloned()
                .unwrap_or_else(AbsolutePath::root);
            display(
                &joined(&place.dir, &[name.as_str()]),
                &[&home, &settled(&home)],
            )
        };
        let deadline = Instant::now() + wait;
        let mut waited = false;
        loop {
            match fs::openat(
                dir,
                name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(file) => return Self::claim(dir, name, file),
                Err(Errno::EXIST) => {
                    if Instant::now() >= deadline {
                        return Err(StateError::LockHeld(shown()));
                    }
                    if !waited {
                        observe(Stage::LockBusy);
                    }
                    waited = true;
                    std::thread::sleep(POLL);
                }
                Err(errno) => return Err(StateError::LockUnusable(shown(), errno)),
            }
        }
    }

    /// Writes the process id; a lock file that cannot be written is ours (`O_EXCL`) but unusable,
    /// so it is not left behind.
    fn claim(dir: &'a OwnedFd, name: String, file: OwnedFd) -> Result<Self, StateError> {
        let lock = Self { dir, name, file };
        let mut handle = std::fs::File::from(lock.file.try_clone()?);
        writeln!(handle, "{}", std::process::id())?;
        Ok(lock)
    }
}

impl Drop for Lock<'_> {
    fn drop(&mut self) {
        let ours = match (
            fs::fstat(&self.file),
            fs::statat(self.dir, self.name.as_str(), AtFlags::SYMLINK_NOFOLLOW),
        ) {
            (Ok(mine), Ok(now)) => same_file(&mine, &now),
            (Err(_), _) | (_, Err(_)) => false,
        };
        if ours {
            // A lock file that cannot be removed is reported by the next run; nothing is forced,
            // and a destructor cannot fail.
            match fs::unlinkat(self.dir, self.name.as_str(), AtFlags::empty()) {
                Ok(()) | Err(_) => {}
            }
        }
    }
}

/// A temporary file in the folder (`O_EXCL|O_NOFOLLOW`, mode `0600` whatever the umask), written
/// in full, flushed and renamed over the file. Any failure removes the temporary file.
fn write_through_temp(
    dir: &OwnedFd,
    place: &Place,
    text: &str,
    expected: Option<&Stat>,
    observe: &dyn Fn(Stage<'_>),
) -> Result<(), StateError> {
    let temp = format!(
        ".{}.{}-{:08x}.tmp",
        place.name,
        std::process::id(),
        RandomState::new().build_hasher().finish() & 0xFFFF_FFFF
    );
    let outcome = fill_temp(dir, &temp, text).and_then(|()| {
        observe(Stage::TempWritten(
            &PathBuf::from(place.dir.as_str()).join(&temp),
        ));
        // A link's target was checked when it was read: it must still be that file.
        match expected {
            Some(read) => {
                let now = fs::statat(dir, place.name.as_str(), AtFlags::SYMLINK_NOFOLLOW)?;
                if same_file(read, &now)
                    && FileType::from_raw_mode(now.st_mode) == FileType::RegularFile
                {
                    Ok(())
                } else {
                    Err(StateError::TargetChanged)
                }
            }
            None => Ok(()),
        }?;
        fs::renameat(dir, temp.as_str(), dir, place.name.as_str()).map_err(StateError::from)
    });
    if outcome.is_err() {
        // The temporary file may never have been created; nothing else can be done about it.
        match fs::unlinkat(dir, temp.as_str(), AtFlags::empty()) {
            Ok(()) | Err(_) => {}
        }
    }
    outcome
}

fn fill_temp(dir: &OwnedFd, temp: &str, text: &str) -> Result<(), StateError> {
    let file = fs::openat(
        dir,
        temp,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    fs::fchmod(&file, Mode::RUSR | Mode::WUSR)?;
    let mut handle = std::fs::File::from(file);
    handle.write_all(text.as_bytes())?;
    handle.sync_all()?;
    Ok(())
}
