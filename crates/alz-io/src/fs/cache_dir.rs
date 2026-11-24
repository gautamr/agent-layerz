//! The capability for the layer cache root (spec §6 "The layer cache", decision-27; TASK-52,
//! TASK-53). The root is found by the component walk, so a link in its own path is followed with
//! each prefix checked, and a root that touches an account folder is refused before anything in
//! it is looked at. Nothing inside the cache is ever followed through a link.

use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicU64, Ordering};

use alz_core::account::{AccountFolders, IndexEntry, lies_within};
use alz_core::cache::{CacheBase, CacheLocation, DirKey, EntryName};
use alz_core::lock::ObjectId;
use alz_core::path::{AbsolutePath, Name, SafeRelPath};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags};
use rustix::io::Errno;

use crate::fs::cache_error::{CacheError, IndexError};
use crate::fs::dir::Dir;
use crate::fs::read_below::read_segments_below;
use crate::fs::read_error::ReadError;
use crate::fs::walk::find;
use crate::fs::walk_error::WalkError;
use crate::fs::write_below::write_file_below;

/// Counts the temporary names this process has made.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// How often a temporary name is tried when one is already taken.
const TEMP_ATTEMPTS: u8 = 16;

/// The longest content of an index entry: a SHA-256 id.
const INDEX_CAP: usize = 64;

/// The repository being built, as the command knows it: the path as written and its real path.
/// An absolute `XDG_CACHE_HOME` inside either is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPaths {
    as_written: AbsolutePath,
    real: AbsolutePath,
}

impl RepositoryPaths {
    /// The repository at `as_written`, whose real path is `real`.
    #[must_use]
    pub const fn new(as_written: AbsolutePath, real: AbsolutePath) -> Self {
        Self { as_written, real }
    }

    /// The repository's path as written.
    #[must_use]
    pub const fn as_written(&self) -> &AbsolutePath {
        &self.as_written
    }

    /// The repository's real path.
    #[must_use]
    pub const fn real(&self) -> &AbsolutePath {
        &self.real
    }
}

/// A handle on the layer cache root: the only way to reach an entry in it.
#[derive(Debug)]
pub struct CacheDir {
    handle: OwnedFd,
    real: AbsolutePath,
}

impl CacheDir {
    /// Opens the cache root at `location`, creating each missing folder of it with mode `0700`
    /// (an existing folder keeps its mode).
    ///
    /// The root is refused, before anything in it is looked at, when it touches an account folder
    /// (§9.1), and, for an absolute `XDG_CACHE_HOME`, when it lies inside the repository, as
    /// written and as a real path. A refusal creates no folder.
    ///
    /// # Errors
    ///
    /// [`CacheError::TouchesAccountFolder`], [`CacheError::InsideRepository`],
    /// [`CacheError::NotPrivate`] when the root is not owned by the effective user or can be
    /// written by group or others (checked with `fstat` on the handle that is kept),
    /// [`CacheError::Unusable`] when the root cannot be opened, and [`CacheError::Io`] when a
    /// folder cannot be created.
    pub fn open_or_create(
        location: &CacheLocation,
        folders: &AccountFolders,
        repository: &RepositoryPaths,
    ) -> Result<Self, CacheError> {
        refuse_inside(location, location.path(), &repository.as_written)?;
        let found = find(location.path(), folders).map_err(|error| match error {
            WalkError::Refused(_) => CacheError::TouchesAccountFolder,
            failure @ (WalkError::Missing
            | WalkError::NotADirectory
            | WalkError::TooManyLinks
            | WalkError::Io(_)) => CacheError::Unusable(failure),
        })?;
        let real = joined(&found.existing, &found.below)?;
        refuse_inside(location, &real, &repository.real)?;
        let handle = found
            .below
            .iter()
            .try_fold(found.handle, |parent, name| make_folder(&parent, name))?;
        let status = fs::fstat(&handle).map_err(|errno| CacheError::Io(errno.into()))?;
        if is_private(status.st_uid, Mode::from_raw_mode(status.st_mode)) {
            Ok(Self { handle, real })
        } else {
            Err(CacheError::NotPrivate)
        }
    }

    /// The real path of the cache root, for a program that has to be given a path (`git clone`).
    #[must_use]
    pub const fn real_path(&self) -> &AbsolutePath {
        &self.real
    }

    /// The entry called `name`, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// [`CacheError::EntryNotDirectory`] when it is a link or anything else but a folder, which
    /// is never followed, [`CacheError::EntryNotOwned`] when the folder opened is not owned by
    /// the effective user (checked with `fstat` on that handle), and [`CacheError::Io`] for any
    /// other failure.
    pub fn entry(&self, name: &EntryName) -> Result<Option<Dir>, CacheError> {
        entry_below(&self.handle, name)
    }

    /// The content of the index entry for the pinned `path:` layer at `dir` and `commit`: the
    /// tree id it names. Only a regular file whose whole content is a lowercase object id counts;
    /// a missing entry, a link, a folder or any other content is a miss, `None`.
    #[must_use]
    pub fn read_index(&self, dir: DirKey, commit: ObjectId) -> Option<ObjectId> {
        match index_below(&self.handle, dir, commit) {
            IndexEntry::Tree(tree) => Some(tree),
            IndexEntry::Missing | IndexEntry::Invalid | IndexEntry::NotOwned => None,
        }
    }

    /// Writes the index entry for `dir` and `commit` with `tree` as its whole content, through a
    /// temporary file made exclusively with mode `0600` and a rename. An entry that already holds
    /// `tree` is left alone, and one that differs is replaced.
    ///
    /// # Errors
    ///
    /// [`IndexError::Blocked`] when a folder is at the name, and [`IndexError::Unwritable`] for
    /// any other failure, such as a read-only cache.
    pub fn write_index(
        &self,
        dir: DirKey,
        commit: ObjectId,
        tree: ObjectId,
    ) -> Result<(), IndexError> {
        let name = index_name(dir, commit);
        let text = name.to_string();
        let unwritable = |errno: Errno| IndexError::Unwritable(name, errno.into());
        if self.read_index(dir, commit) == Some(tree) {
            return Ok(());
        }
        match fs::statat(&self.handle, text.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
            Ok(status) if FileType::from_raw_mode(status.st_mode) == FileType::Directory => {
                return Err(IndexError::Blocked(name));
            }
            Ok(_) | Err(Errno::NOENT) => {}
            Err(errno) => return Err(unwritable(errno)),
        }
        let temporary = temporary_name();
        write_exclusively(&self.handle, &temporary, tree.to_string().as_bytes())
            .map_err(|error| IndexError::Unwritable(name, error))?;
        fs::renameat(
            &self.handle,
            temporary.as_str(),
            &self.handle,
            text.as_str(),
        )
        .map_err(|errno| {
            // The temporary file is ours; a failed removal leaves a `.tmp-` file that no
            // reader looks at.
            fs::unlinkat(&self.handle, temporary.as_str(), AtFlags::empty()).unwrap_or_default();
            match errno {
                Errno::ISDIR => IndexError::Blocked(name),
                other => unwritable(other),
            }
        })
    }
}

/// The entry called `name` below the cache root `root`, opened with `O_NOFOLLOW` and checked with
/// `fstat` on that handle: a folder owned by the effective user. `None` when there is none.
pub(super) fn entry_below(root: &OwnedFd, name: &EntryName) -> Result<Option<Dir>, CacheError> {
    match open_folder(root, &name.to_string()) {
        Ok(handle) => {
            let status = fs::fstat(&handle).map_err(|errno| CacheError::Io(errno.into()))?;
            if status.st_uid == rustix::process::geteuid().as_raw() {
                Ok(Some(Dir::from_handle(handle)))
            } else {
                Err(CacheError::EntryNotOwned(*name))
            }
        }
        Err(Errno::NOENT) => Ok(None),
        Err(Errno::LOOP | Errno::NOTDIR) => Err(CacheError::EntryNotDirectory(*name)),
        Err(errno) => Err(CacheError::Io(errno.into())),
    }
}

/// What the index entry for `dir` and `commit` below the cache root `root` holds: only a regular
/// file that this user owns and whose whole content is a lowercase object id is a tree.
pub(super) fn index_below(root: &OwnedFd, dir: DirKey, commit: ObjectId) -> IndexEntry {
    let name = index_name(dir, commit).to_string();
    match read_segments_below(root, &[name.as_str()], INDEX_CAP) {
        Err(ReadError::Missing) => IndexEntry::Missing,
        Err(ReadError::NotRegular | ReadError::TooLarge | ReadError::Io(_)) => IndexEntry::Invalid,
        Ok(read) => index_read(
            read.owner(),
            read.into_bytes(),
            rustix::process::geteuid().as_raw(),
        ),
    }
}

/// What a regular index file owned by `owner` holds, for the user `user`: a file that another user
/// owns is never read, whatever it holds.
fn index_read(owner: u32, bytes: Vec<u8>, user: u32) -> IndexEntry {
    if owner == user {
        String::from_utf8(bytes)
            .ok()
            .and_then(|text| ObjectId::try_from(text.as_str()).ok())
            .map_or(IndexEntry::Invalid, IndexEntry::Tree)
    } else {
        IndexEntry::NotOwned
    }
}

/// Whether a folder with owner `uid` and `mode` is private to this process's user: owned by the
/// effective user, and not writable by group or others.
pub(super) fn is_private(uid: u32, mode: Mode) -> bool {
    uid == rustix::process::geteuid().as_raw() && !mode.intersects(Mode::WGRP | Mode::WOTH)
}

fn index_name(dir: DirKey, commit: ObjectId) -> EntryName {
    EntryName::PathIndex { dir, commit }
}

/// Refuses a cache root that is inside the repository when an absolute `XDG_CACHE_HOME` put it
/// there.
fn refuse_inside(
    location: &CacheLocation,
    root: &AbsolutePath,
    repository: &AbsolutePath,
) -> Result<(), CacheError> {
    match location.base() {
        CacheBase::XdgCacheHome if lies_within(root, repository) => {
            Err(CacheError::InsideRepository)
        }
        CacheBase::XdgCacheHome | CacheBase::Home => Ok(()),
    }
}

/// `existing` with the `below` names added.
pub(super) fn joined(existing: &AbsolutePath, below: &[Name]) -> Result<AbsolutePath, CacheError> {
    let parts = existing
        .components()
        .chain(below.iter().map(Name::as_str))
        .collect::<Vec<_>>();
    let text = format!("/{}", parts.join("/"));
    AbsolutePath::try_from(text.as_str())
        .map_err(|_not_absolute| CacheError::Io(io::Error::other("not an absolute path")))
}

/// Creates the folder `name` below `parent` with mode `0700` and opens it. A folder that is there
/// already (another alz made it) is opened; one that is a link is refused by `O_NOFOLLOW`.
fn make_folder(parent: &OwnedFd, name: &Name) -> Result<OwnedFd, CacheError> {
    match fs::mkdirat(parent, name.as_str(), Mode::RWXU) {
        Ok(()) | Err(Errno::EXIST) => {}
        Err(errno) => return Err(CacheError::Io(errno.into())),
    }
    open_folder(parent, name.as_str()).map_err(|errno| CacheError::Io(errno.into()))
}

pub(super) fn open_folder(parent: &OwnedFd, name: &str) -> Result<OwnedFd, Errno> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
}

/// `.tmp-<pid>-<n>`: a name no entry has, because entries start with `git-`, `tree2@` or
/// `pathref@`.
fn temporary_name() -> String {
    format!(
        ".tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// Writes `bytes` to a new file `name` below `parent`: it must not exist (`O_EXCL`), is never
/// reached through a link (`O_NOFOLLOW`) and has mode `0600`.
fn write_exclusively(parent: &OwnedFd, name: &str, bytes: &[u8]) -> io::Result<()> {
    let file = fs::openat(
        parent,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    let mut file = std::fs::File::from(file);
    file.write_all(bytes)?;
    file.sync_all()
}

/// A folder made in the cache root to be filled and then published under its final name.
///
/// It is `.tmp-<pid>-<n>`, made with an exclusive `mkdirat` with mode `0700`, and its handle
/// is checked with `fstat`: a directory owned by this process's user. It is removed with
/// everything in it, without following a link, when it is dropped, unless it was published.
#[derive(Debug)]
pub struct TempEntry<'c> {
    cache: &'c CacheDir,
    name: String,
    dir: Dir,
    published: bool,
}

impl<'c> TempEntry<'c> {
    /// Makes a new temporary entry in `cache`.
    ///
    /// # Errors
    ///
    /// [`CacheError::TempEntryReplaced`] when the name was swapped before it was opened, and
    /// [`CacheError::Io`] for any other failure.
    pub fn create(cache: &'c CacheDir) -> Result<Self, CacheError> {
        let name = (0..TEMP_ATTEMPTS)
            .map(|_| temporary_name())
            .find_map(
                |name| match fs::mkdirat(&cache.handle, name.as_str(), Mode::RWXU) {
                    Ok(()) => Some(Ok(name)),
                    Err(Errno::EXIST) => None,
                    Err(errno) => Some(Err(CacheError::Io(errno.into()))),
                },
            )
            .unwrap_or_else(|| Err(CacheError::Io(io::Error::other("no free temporary name"))))?;
        let handle = open_folder(&cache.handle, &name).map_err(|errno| match errno {
            Errno::LOOP | Errno::NOTDIR => CacheError::TempEntryReplaced,
            other => CacheError::Io(other.into()),
        })?;
        let status = fs::fstat(&handle).map_err(|errno| CacheError::Io(errno.into()))?;
        if FileType::from_raw_mode(status.st_mode) == FileType::Directory
            && status.st_uid == rustix::process::geteuid().as_raw()
        {
            Ok(Self {
                cache,
                name,
                dir: Dir::from_handle(handle),
                published: false,
            })
        } else {
            // Not ours: the name was swapped, so nothing is removed under it.
            Err(CacheError::TempEntryReplaced)
        }
    }

    /// The name of the entry, `.tmp-<pid>-<n>`, inside the cache root.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The handle on the entry, to read what was written there.
    #[must_use]
    pub const fn dir(&self) -> &Dir {
        &self.dir
    }

    /// Writes the file `path` in the entry, with its folders, and sets its mode to `0644`, or
    /// `0755` when `executable`, whatever the umask. Nothing is followed and nothing is replaced:
    /// a name that exists is an error.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] for any failure, such as a file that is there already.
    pub fn write_file(
        &self,
        path: &SafeRelPath,
        bytes: &[u8],
        executable: bool,
    ) -> Result<(), CacheError> {
        write_file_below(self.dir.handle(), path, bytes, executable).map_err(CacheError::Io)
    }

    /// Moves the entry to `name` without replacing anything: an entry that is there already stays
    /// as it is, and is returned to be checked like a cached one, and this entry is removed.
    ///
    /// # Errors
    ///
    /// [`CacheError::EntryNotDirectory`] when what is there is a link or not a folder, and
    /// [`CacheError::Io`] for any other failure.
    pub fn publish(mut self, name: &EntryName) -> Result<Published, CacheError> {
        let target = name.to_string();
        match rename_without_replacing(&self.cache.handle, self.name.as_str(), target.as_str()) {
            Ok(()) => {
                self.published = true;
                Ok(Published::Created(self.dir.try_clone()?))
            }
            Err(Errno::EXIST | Errno::NOTEMPTY) => self
                .cache
                .entry(name)?
                .map(Published::Existing)
                .ok_or(CacheError::EntryVanished(*name)),
            Err(errno) => Err(CacheError::Io(errno.into())),
        }
    }
}

impl Drop for TempEntry<'_> {
    fn drop(&mut self) {
        if !self.published
            && let Ok(name) = CString::new(self.name.as_str())
        {
            // Best effort: a destructor never fails, and a `.tmp-` folder that stays is never read.
            remove_tree(&self.cache.handle, &name).unwrap_or_default();
        }
    }
}

/// Moves the folder `from` to `to` below `parent`, and never replaces what is at `to`. The file
/// system's own refusal to replace is used (`RENAME_NOREPLACE`). A file system that does not
/// know the flag (`EINVAL`, `ENOSYS`, `EOPNOTSUPP`) is served by a plain rename, which stays free
/// of a race for a folder: the rename is one atomic step, and it replaces nothing but an empty
/// folder and fails with `ENOTEMPTY` on any other folder and with `ENOTDIR` on a name that is not
/// a folder. An empty folder is no published entry, since every one holds the files of a layer.
fn rename_without_replacing(parent: &OwnedFd, from: &str, to: &str) -> Result<(), Errno> {
    match fs::renameat_with(parent, from, parent, to, RenameFlags::NOREPLACE) {
        Err(Errno::INVAL | Errno::NOSYS | Errno::OPNOTSUPP) => {
            fs::renameat(parent, from, parent, to)
        }
        other => other,
    }
}

/// What publishing a temporary entry did.
#[derive(Debug)]
pub enum Published {
    /// The entry was moved to its name; this is its handle.
    Created(Dir),
    /// An entry was there already and was left as it is; this is its handle, to be checked.
    Existing(Dir),
}

/// Which folder a handle is on, by device and inode: what `..` is checked against on the way up.
type FolderId = (i128, i128);

fn identity(folder: &OwnedFd) -> Result<FolderId, Errno> {
    fs::fstat(folder).map(|status| (i128::from(status.st_dev), i128::from(status.st_ino)))
}

fn open_directory(parent: &OwnedFd, name: &CStr) -> Result<OwnedFd, Errno> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
}

/// A folder on the way down in [`remove_tree`]: its name in its parent, which folder its parent is
/// (`None` for the folder `remove_tree` was given), and the folders in it that are yet to be
/// removed.
struct Level {
    name: CString,
    parent: Option<FolderId>,
    pending: Vec<CString>,
}

impl Level {
    /// Enters `folder`: every entry that is not a folder is removed at once, a link as a link, and
    /// the folders are kept to be entered one at a time.
    fn enter(folder: &OwnedFd, name: CString, parent: Option<FolderId>) -> Result<Self, Errno> {
        let names = fs::Dir::read_from(folder)?
            .map(|entry| entry.map(|entry| entry.file_name().to_owned()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pending = Vec::new();
        for child in names
            .into_iter()
            .filter(|child| !matches!(child.to_bytes(), b"." | b".."))
        {
            let status = fs::statat(folder, child.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)?;
            match FileType::from_raw_mode(status.st_mode) {
                FileType::Directory => pending.push(child),
                FileType::RegularFile
                | FileType::Symlink
                | FileType::Fifo
                | FileType::Socket
                | FileType::CharacterDevice
                | FileType::BlockDevice
                | FileType::Unknown => fs::unlinkat(folder, child.as_c_str(), AtFlags::empty())?,
            }
        }
        Ok(Self {
            name,
            parent,
            pending,
        })
    }
}

/// Removes the folder `name` below `parent` and everything in it, never following a link: a link
/// is removed as a link, and a folder is entered only through `O_NOFOLLOW`.
///
/// The walk is a loop over an explicit stack of names, so the depth of the tree costs neither call
/// frames nor open handles: one handle is on the folder being emptied and one more is open for the
/// moment of going up. Going up opens `..` and checks that it is the folder that was left from
/// (device and inode), so a rename of the tree meanwhile removes nothing it should not.
fn remove_tree(parent: &OwnedFd, name: &CStr) -> Result<(), Errno> {
    let mut current = open_directory(parent, name)?;
    let mut levels = vec![Level::enter(&current, name.to_owned(), None)?];
    loop {
        let Some(level) = levels.last_mut() else {
            return Ok(());
        };
        if let Some(child) = level.pending.pop() {
            let from = identity(&current)?;
            let below = open_directory(&current, child.as_c_str())?;
            levels.push(Level::enter(&below, child, Some(from))?);
            current = below;
            continue;
        }
        // Nothing is left in this folder: it is removed from the folder above it.
        let Some(done) = levels.pop() else {
            return Ok(());
        };
        let Some(expected) = done.parent else {
            return fs::unlinkat(parent, done.name.as_c_str(), AtFlags::REMOVEDIR);
        };
        let above = open_directory(&current, c"..")?;
        if identity(&above)? != expected {
            return Err(Errno::STALE);
        }
        fs::unlinkat(&above, done.name.as_c_str(), AtFlags::REMOVEDIR)?;
        current = above;
    }
}

impl From<ReadError> for CacheError {
    fn from(error: ReadError) -> Self {
        Self::Io(io::Error::other(error))
    }
}

#[cfg(test)]
mod tests {
    use rustix::fs::Mode;

    use std::os::fd::OwnedFd;

    use alz_test_support::ScratchDir;
    use rustix::fs::{self, AtFlags, OFlags};

    use alz_core::account::IndexEntry;
    use alz_core::lock::ObjectId;

    use super::{index_read, is_private, open_directory, remove_tree, rename_without_replacing};

    /// A handle on the scratch folder, to remove folders below.
    fn open_scratch(scratch: &ScratchDir) -> OwnedFd {
        fs::open(
            scratch.path(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .expect("opened")
    }

    #[test]
    fn a_tree_far_deeper_than_the_open_file_limit_is_removed_with_two_handles() {
        let scratch = ScratchDir::new().expect("a scratch folder");
        let root = open_scratch(&scratch);
        fs::mkdirat(&root, c"deep", Mode::RWXU).expect("made");
        // Each level holds a file; only the handle on the deepest folder is kept while building.
        let deepest = (0..600).try_fold(
            open_directory(&root, c"deep").expect("opened"),
            |folder, _| {
                fs::mkdirat(&folder, c"d", Mode::RWXU)?;
                fs::openat(&folder, c"f", OFlags::WRONLY | OFlags::CREATE, Mode::RUSR)?;
                open_directory(&folder, c"d")
            },
        );
        assert!(deepest.is_ok(), "{deepest:?}");
        drop(deepest);
        remove_tree(&root, c"deep").expect("removed");
        assert!(!scratch.exists("deep"));
    }

    #[test]
    fn a_link_in_the_tree_is_removed_as_a_link_and_what_it_names_stays() {
        let scratch = ScratchDir::new().expect("a scratch folder");
        scratch.make_dirs("outside/inner").expect("made");
        scratch.write("outside/inner/keep", "x").expect("written");
        scratch.make_dirs("tree/a/b").expect("made");
        scratch.write("tree/a/b/file", "x").expect("written");
        scratch
            .make_link("tree/a/link", scratch.join("outside"))
            .expect("linked");
        scratch.make_link("tree/b-link", "a").expect("linked");
        remove_tree(&open_scratch(&scratch), c"tree").expect("removed");
        assert!(!scratch.exists("tree"));
        assert!(scratch.is_file("outside/inner/keep"));
    }

    #[test]
    fn a_folder_is_moved_and_never_replaces_a_folder_that_holds_files_or_a_name_that_is_a_file() {
        let scratch = ScratchDir::new().expect("a scratch folder");
        scratch.make_dirs("from").expect("made");
        scratch.write("from/a", "new").expect("written");
        scratch.make_dirs("held").expect("made");
        scratch.write("held/a", "old").expect("written");
        scratch.write("file", "old").expect("written");
        let root = open_scratch(&scratch);
        for target in ["held", "file"] {
            assert!(
                rename_without_replacing(&root, "from", target).is_err(),
                "{target}"
            );
            assert_eq!(scratch.read_text("from/a").expect("kept"), "new");
        }
        assert_eq!(scratch.read_text("held/a").expect("kept"), "old");
        assert_eq!(scratch.read_text("file").expect("kept"), "old");
        // The plain rename that serves a file system without the flag behaves the same way.
        assert_eq!(
            fs::renameat(&root, "from", &root, "held"),
            Err(rustix::io::Errno::NOTEMPTY)
        );
        assert_eq!(
            fs::renameat(&root, "from", &root, "file"),
            Err(rustix::io::Errno::NOTDIR)
        );
        rename_without_replacing(&root, "from", "free").expect("moved");
        assert_eq!(scratch.read_text("free/a").expect("moved"), "new");
    }

    #[test]
    fn a_name_that_is_a_link_is_not_entered() {
        let scratch = ScratchDir::new().expect("a scratch folder");
        scratch.make_dirs("outside").expect("made");
        scratch.write("outside/keep", "x").expect("written");
        scratch
            .make_link("tree", scratch.join("outside"))
            .expect("linked");
        assert!(remove_tree(&open_scratch(&scratch), c"tree").is_err());
        assert!(scratch.is_file("outside/keep"));
        fs::unlinkat(open_scratch(&scratch), c"tree", AtFlags::empty()).expect("unlinked");
    }

    #[test]
    fn an_index_file_that_another_user_owns_is_not_read_whatever_it_holds() {
        let tree = "0123456789abcdef0123456789abcdef01234567";
        let expected = ObjectId::try_from(tree).expect("an id");
        assert_eq!(
            index_read(1000, tree.as_bytes().to_vec(), 1000),
            IndexEntry::Tree(expected)
        );
        assert_eq!(
            index_read(0, tree.as_bytes().to_vec(), 1000),
            IndexEntry::NotOwned
        );
        assert_eq!(
            index_read(1000, b"not an id".to_vec(), 1000),
            IndexEntry::Invalid
        );
    }

    #[test]
    fn a_folder_is_private_when_this_user_owns_it_and_neither_group_nor_others_can_write() {
        let own = rustix::process::geteuid().as_raw();
        let other = own.wrapping_add(1);
        assert!(is_private(own, Mode::from_raw_mode(0o700)));
        assert!(is_private(own, Mode::from_raw_mode(0o755)));
        assert!(!is_private(own, Mode::from_raw_mode(0o770)));
        assert!(!is_private(own, Mode::from_raw_mode(0o707)));
        assert!(!is_private(other, Mode::from_raw_mode(0o700)));
    }
}
