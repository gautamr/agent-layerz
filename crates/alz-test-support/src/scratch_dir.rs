//! A directory made for one test and removed when the value is dropped.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts the directories this process has made, so two tests never share a name.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory under the system's temporary directory, removed with everything in it
/// when the value is dropped. The name holds the process id and a counter, and the directory is
/// created exclusively, so a stale or planted path of the same name is an error and not reused.
#[derive(Debug)]
pub struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Makes the directory.
    ///
    /// # Errors
    ///
    /// The error of creating the directory.
    pub fn new() -> io::Result<Self> {
        let name = format!(
            "alz-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path of `name` inside the directory.
    #[must_use]
    pub fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.path.join(name)
    }

    /// Writes a regular file `name` with `contents`, replacing any file of that name.
    ///
    /// # Errors
    ///
    /// The error of writing the file.
    pub fn write(&self, name: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
        fs::write(self.join(name), contents)
    }

    /// Writes an executable file `name` (mode 0755) with `contents`.
    ///
    /// # Errors
    ///
    /// The error of writing the file or of setting its mode.
    pub fn write_executable(
        &self,
        name: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
    ) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let path = self.join(name);
        fs::write(&path, contents)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
    }

    /// The bytes of the file `name`.
    ///
    /// # Errors
    ///
    /// The error of reading the file.
    pub fn read(&self, name: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        fs::read(self.join(name))
    }

    /// Whether `name` exists inside the directory, links not followed.
    #[must_use]
    pub fn exists(&self, name: impl AsRef<Path>) -> bool {
        fs::symlink_metadata(self.join(name)).is_ok()
    }

    /// Makes a folder `name`.
    ///
    /// # Errors
    ///
    /// The error of creating the folder.
    pub fn make_dir(&self, name: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir(self.join(name))
    }

    /// Makes the folder `name` and the folders above it that are missing.
    ///
    /// # Errors
    ///
    /// The error of creating the folders.
    pub fn make_dirs(&self, name: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(self.join(name))
    }

    /// The real path of the directory itself, every link resolved.
    ///
    /// # Errors
    ///
    /// The error of resolving the path.
    pub fn real_path(&self) -> io::Result<PathBuf> {
        fs::canonicalize(&self.path)
    }

    /// Reads the file `name` as UTF-8 text.
    ///
    /// # Errors
    ///
    /// The error of reading the file, or of the text not being UTF-8.
    pub fn read_text(&self, name: impl AsRef<Path>) -> io::Result<String> {
        fs::read_to_string(self.join(name))
    }

    /// Removes the folder `name` with everything in it.
    ///
    /// # Errors
    ///
    /// The error of removing it.
    pub fn remove_tree(&self, name: impl AsRef<Path>) -> io::Result<()> {
        fs::remove_dir_all(self.join(name))
    }

    /// Renames `from` to `to`, both inside the directory.
    ///
    /// # Errors
    ///
    /// The error of renaming.
    pub fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        fs::rename(self.join(from), self.join(to))
    }

    /// Sets the permission bits of `name`, such as `0o500`.
    ///
    /// # Errors
    ///
    /// The error of setting them.
    pub fn set_mode(&self, name: impl AsRef<Path>, mode: u32) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(self.join(name), fs::Permissions::from_mode(mode))
    }

    /// Whether `name` is a folder, links followed.
    #[must_use]
    pub fn is_dir(&self, name: impl AsRef<Path>) -> bool {
        fs::metadata(self.join(name)).is_ok_and(|meta| meta.is_dir())
    }

    /// Whether `name` is a regular file, links followed.
    #[must_use]
    pub fn is_file(&self, name: impl AsRef<Path>) -> bool {
        fs::metadata(self.join(name)).is_ok_and(|meta| meta.is_file())
    }

    /// The names inside the folder `name`, sorted; none when it cannot be listed.
    #[must_use]
    pub fn list(&self, name: impl AsRef<Path>) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.join(name))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Makes a symbolic link `name` that points at `target`, which is stored as it is given.
    ///
    /// # Errors
    ///
    /// The error of creating the link.
    pub fn make_link(&self, name: impl AsRef<Path>, target: impl AsRef<Path>) -> io::Result<()> {
        std::os::unix::fs::symlink(target, self.join(name))
    }

    /// Makes a FIFO (named pipe) `name`. Opening it for reading blocks until a writer opens it,
    /// unless the open is non-blocking.
    ///
    /// # Errors
    ///
    /// The error of running `mkfifo`, or its failure status.
    pub fn make_fifo(&self, name: impl AsRef<Path>) -> io::Result<()> {
        let status = Command::new("mkfifo").arg(self.join(name)).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other("mkfifo failed"))
        }
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // A directory that cannot be removed is left to the system's temporary-file cleanup;
        // a destructor never fails.
        fs::remove_dir_all(&self.path).unwrap_or_default();
    }
}
