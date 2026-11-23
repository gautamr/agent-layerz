//! The capability for the layer cache root (spec §6 "The layer cache", decision-27; TASK-52,
//! TASK-53). The root is found by the component walk, so a link in its own path is followed with
//! each prefix checked, and a root that touches an account folder is refused before anything in
//! it is looked at. Nothing inside the cache is ever followed through a link.

use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicU64, Ordering};

use alz_core::account::{AccountFolders, lies_within};
use alz_core::cache::{CacheBase, CacheLocation, DirKey, EntryName};
use alz_core::lock::ObjectId;
use alz_core::path::{AbsolutePath, Name, SafeRelPath};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags};
use rustix::io::Errno;

use crate::fs::cache_error::{CacheError, IndexError};
use crate::fs::dir::Dir;
use crate::fs::read_below::read_file_below;
use crate::fs::read_error::ReadError;
use crate::fs::walk::find;
use crate::fs::walk_error::WalkError;

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
        Ok(Self { handle, real })
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
    /// is never followed, and [`CacheError::Io`] for any other failure.
    pub fn entry(&self, name: &EntryName) -> Result<Option<Dir>, CacheError> {
        match open_folder(&self.handle, &name.to_string()) {
            Ok(handle) => Ok(Some(Dir::from_handle(handle))),
            Err(Errno::NOENT) => Ok(None),
            Err(Errno::LOOP | Errno::NOTDIR) => Err(CacheError::EntryNotDirectory(*name)),
            Err(errno) => Err(CacheError::Io(errno.into())),
        }
    }

    /// The content of the index entry for the pinned `path:` layer at `dir` and `commit`: the
    /// tree id it names. Only a regular file whose whole content is a lowercase object id counts;
    /// a missing entry, a link, a folder or any other content is a miss, `None`.
    #[must_use]
    pub fn read_index(&self, dir: DirKey, commit: ObjectId) -> Option<ObjectId> {
        let path = SafeRelPath::try_from(index_name(dir, commit).to_string().as_str()).ok()?;
        read_file_below(&self.handle, &path, INDEX_CAP)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| ObjectId::try_from(text.as_str()).ok())
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
fn joined(existing: &AbsolutePath, below: &[Name]) -> Result<AbsolutePath, CacheError> {
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

fn open_folder(parent: &OwnedFd, name: &str) -> Result<OwnedFd, Errno> {
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

    /// Moves the entry to `name` without replacing anything: an entry that is there already stays
    /// as it is, and is returned to be checked like a cached one, and this entry is removed.
    ///
    /// # Errors
    ///
    /// [`CacheError::EntryNotDirectory`] when what is there is a link or not a folder, and
    /// [`CacheError::Io`] for any other failure.
    pub fn publish(mut self, name: &EntryName) -> Result<Published, CacheError> {
        let target = name.to_string();
        match fs::renameat_with(
            &self.cache.handle,
            self.name.as_str(),
            &self.cache.handle,
            target.as_str(),
            RenameFlags::NOREPLACE,
        ) {
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

/// What publishing a temporary entry did.
#[derive(Debug)]
pub enum Published {
    /// The entry was moved to its name; this is its handle.
    Created(Dir),
    /// An entry was there already and was left as it is; this is its handle, to be checked.
    Existing(Dir),
}

/// Removes the folder `name` below `parent` and everything in it, never following a link: a link
/// is removed as a link, and a folder is entered only through `O_NOFOLLOW`.
fn remove_tree(parent: &OwnedFd, name: &CStr) -> Result<(), Errno> {
    let folder = fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let names = fs::Dir::read_from(&folder)?
        .map(|entry| entry.map(|entry| entry.file_name().to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    for child in names
        .iter()
        .filter(|child| !matches!(child.to_bytes(), b"." | b".."))
    {
        let status = fs::statat(&folder, child.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        match FileType::from_raw_mode(status.st_mode) {
            FileType::Directory => remove_tree(&folder, child)?,
            FileType::RegularFile
            | FileType::Symlink
            | FileType::Fifo
            | FileType::Socket
            | FileType::CharacterDevice
            | FileType::BlockDevice
            | FileType::Unknown => fs::unlinkat(&folder, child.as_c_str(), AtFlags::empty())?,
        }
    }
    fs::unlinkat(parent, name, AtFlags::REMOVEDIR)
}

impl From<ReadError> for CacheError {
    fn from(error: ReadError) -> Self {
        Self::Io(io::Error::other(error))
    }
}
