//! A directory that can be read and never written (decision-27, doc-6 §3.4).

use std::fs::File;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::Path;

use alz_core::path::SafeRelPath;
use rustix::fs::{self, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::fs::read_error::ReadError;

/// A handle on a directory the person named, such as the working directory or `--layer <path>`.
/// It has no method that writes, creates or removes anything. A name inside it is opened relative
/// to the handle and never through a link.
#[derive(Debug)]
pub struct ReadOnlyDir {
    handle: OwnedFd,
}

impl ReadOnlyDir {
    /// Opens the directory at `path`. A link in `path` itself is followed, because the person
    /// typed that path; what lies inside the directory is never reached through a link.
    ///
    /// # Errors
    ///
    /// [`ReadError::Missing`] when `path` does not exist or is not a directory, and
    /// [`ReadError::Io`] for any other failure.
    pub fn open(path: &Path) -> Result<Self, ReadError> {
        fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(|handle| Self { handle })
        .map_err(|errno| match errno {
            Errno::NOENT | Errno::NOTDIR => ReadError::Missing,
            other => ReadError::Io(other.into()),
        })
    }

    /// Reads the regular file at `path` below the directory, at most `cap` bytes.
    ///
    /// Every component is opened relative to the handle before it, with `O_NOFOLLOW`, so no
    /// link is followed. The file is opened with `O_NONBLOCK`, so a pipe or a device cannot block
    /// the open, and what was opened is checked with `fstat` on that handle: only a regular file
    /// is read (decision-27, amended). `st_size` is not trusted: at most `cap + 1` bytes are
    /// read, and the file is refused when the last of them arrives.
    ///
    /// # Errors
    ///
    /// [`ReadError::Missing`] when a component does not exist, [`ReadError::NotRegular`] when
    /// the last component is a link, a folder or anything else but a regular file (or an earlier
    /// one is a link or not a folder), [`ReadError::TooLarge`] when the file is longer than
    /// `cap` bytes, and [`ReadError::Io`] for any other failure.
    pub fn read_file(&self, path: &SafeRelPath, cap: usize) -> Result<Vec<u8>, ReadError> {
        let segments: Vec<&str> = path.segments().collect();
        let (name, parents) = segments.split_last().ok_or(ReadError::Missing)?;
        let directory = parents.iter().try_fold(
            self.handle.try_clone().map_err(ReadError::Io)?,
            |directory, segment| {
                open_below(
                    &directory,
                    segment,
                    OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                )
            },
        )?;
        let file = open_below(
            &directory,
            name,
            OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        )?;
        require_regular(&file)?;
        read_at_most(File::from(file), cap)
    }
}

/// Opens `name` read-only relative to `directory`. The refusal of a link surfaces as `ELOOP`
/// (or `ENOTDIR` with `O_DIRECTORY`), which is [`ReadError::NotRegular`].
fn open_below(directory: &OwnedFd, name: &str, flags: OFlags) -> Result<OwnedFd, ReadError> {
    fs::openat(directory, name, OFlags::RDONLY | flags, Mode::empty()).map_err(
        |errno| match errno {
            Errno::NOENT => ReadError::Missing,
            Errno::LOOP | Errno::NOTDIR => ReadError::NotRegular,
            other => ReadError::Io(other.into()),
        },
    )
}

/// The trust check of decision-27: `fstat` on the handle that is read, never on a name.
fn require_regular(file: &OwnedFd) -> Result<(), ReadError> {
    let status = fs::fstat(file).map_err(|errno| ReadError::Io(errno.into()))?;
    match FileType::from_raw_mode(status.st_mode) {
        FileType::RegularFile => Ok(()),
        FileType::Directory
        | FileType::Symlink
        | FileType::Fifo
        | FileType::Socket
        | FileType::CharacterDevice
        | FileType::BlockDevice
        | FileType::Unknown => Err(ReadError::NotRegular),
    }
}

/// Reads `source` to its end, taking at most `cap + 1` bytes, and refuses it when the last of
/// those arrives. The source's own size is never consulted.
fn read_at_most(source: impl Read, cap: usize) -> Result<Vec<u8>, ReadError> {
    let limit = u64::try_from(cap).map_or(u64::MAX, |cap| cap.saturating_add(1));
    let mut bytes = Vec::new();
    source
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(ReadError::Io)?;
    if bytes.len() > cap {
        Err(ReadError::TooLarge)
    } else {
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use alz_core::path::SafeRelPath;
    use alz_test_support::ScratchDir;

    use super::{ReadError, ReadOnlyDir, read_at_most};

    const CAP: usize = 1_048_576;

    fn path(text: &str) -> SafeRelPath {
        SafeRelPath::try_from(text).expect("a safe path")
    }

    fn scratch() -> ScratchDir {
        ScratchDir::new().expect("a scratch directory")
    }

    fn read(dir: &ScratchDir, name: &str) -> Result<Vec<u8>, ReadError> {
        ReadOnlyDir::open(dir.path())?.read_file(&path(name), CAP)
    }

    /// Reads on another thread, so a read that blocks fails the test and never hangs it. `None`
    /// is a read that did not end in ten seconds.
    fn read_within_ten_seconds(dir: &ScratchDir, name: &str) -> Option<Result<Vec<u8>, ReadError>> {
        let (sender, receiver) = mpsc::channel();
        let (root, name) = (dir.path().to_path_buf(), name.to_owned());
        thread::spawn(move || {
            let result = ReadOnlyDir::open(&root).and_then(|dir| dir.read_file(&path(&name), CAP));
            sender.send(result).unwrap_or_default();
        });
        receiver.recv_timeout(Duration::from_secs(10)).ok()
    }

    fn assert_refused_as_not_regular(dir: &ScratchDir, name: &str) {
        let result = read_within_ten_seconds(dir, name).expect("the read must not block");
        assert!(
            matches!(result, Err(ReadError::NotRegular)),
            "result: {result:?}"
        );
    }

    #[test]
    fn a_regular_file_is_read_whole() {
        let dir = scratch();
        dir.write("layer.yaml", "name: x\n").expect("written");
        assert_eq!(read(&dir, "layer.yaml").expect("read"), b"name: x\n");
    }

    #[test]
    fn an_empty_file_is_read() {
        let dir = scratch();
        dir.write("layer.yaml", "").expect("written");
        assert_eq!(read(&dir, "layer.yaml").expect("read"), Vec::<u8>::new());
    }

    #[test]
    fn a_file_of_exactly_the_cap_is_read_and_one_byte_more_is_refused() {
        let dir = scratch();
        dir.write("at", vec![b'a'; CAP]).expect("written");
        dir.write("past", vec![b'a'; CAP + 1]).expect("written");
        assert_eq!(read(&dir, "at").expect("read").len(), CAP);
        assert!(matches!(read(&dir, "past"), Err(ReadError::TooLarge)));
    }

    #[test]
    fn a_file_far_past_the_cap_is_refused() {
        let dir = scratch();
        dir.write("big", vec![b'a'; 3 * CAP]).expect("written");
        assert!(matches!(read(&dir, "big"), Err(ReadError::TooLarge)));
    }

    /// A source that never ends and counts what it hands out.
    struct Endless {
        served: usize,
    }

    impl Read for Endless {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            buffer.fill(b'a');
            self.served += buffer.len();
            Ok(buffer.len())
        }
    }

    #[test]
    fn at_most_the_cap_plus_one_byte_is_read_whatever_the_source_says() {
        let mut source = Endless { served: 0 };
        let result = read_at_most(&mut source, 1000);
        assert!(matches!(result, Err(ReadError::TooLarge)));
        // `take` hands the source buffers no larger than what is left, so nothing beyond the
        // cap plus one byte is ever requested.
        assert_eq!(source.served, 1001);
    }

    #[test]
    fn a_missing_file_is_missing() {
        let dir = scratch();
        assert!(matches!(read(&dir, "layer.yaml"), Err(ReadError::Missing)));
    }

    #[test]
    fn a_link_to_a_regular_file_is_refused() {
        let dir = scratch();
        dir.write("real.yaml", "name: x\n").expect("written");
        dir.make_link("layer.yaml", "real.yaml").expect("linked");
        assert_refused_as_not_regular(&dir, "layer.yaml");
    }

    #[test]
    fn a_link_to_a_missing_file_is_refused_as_a_link_not_as_missing() {
        let dir = scratch();
        dir.make_link("layer.yaml", "nowhere").expect("linked");
        assert_refused_as_not_regular(&dir, "layer.yaml");
    }

    #[test]
    fn a_link_to_a_fifo_is_refused_without_blocking() {
        let dir = scratch();
        dir.make_fifo("pipe").expect("a fifo");
        dir.make_link("layer.yaml", "pipe").expect("linked");
        assert_refused_as_not_regular(&dir, "layer.yaml");
    }

    #[test]
    fn a_folder_is_refused() {
        let dir = scratch();
        dir.make_dir("layer.yaml").expect("made");
        assert_refused_as_not_regular(&dir, "layer.yaml");
    }

    #[test]
    fn a_fifo_is_refused_without_blocking() {
        let dir = scratch();
        dir.make_fifo("layer.yaml").expect("a fifo");
        assert_refused_as_not_regular(&dir, "layer.yaml");
    }

    #[test]
    fn a_link_in_the_middle_of_a_path_is_never_followed() {
        let dir = scratch();
        dir.make_dir("real").expect("made");
        dir.write("real/layer.yaml", "name: x\n").expect("written");
        dir.make_link("alias", "real").expect("linked");
        assert_refused_as_not_regular(&dir, "alias/layer.yaml");
        assert_eq!(read(&dir, "real/layer.yaml").expect("read"), b"name: x\n");
    }

    #[test]
    fn a_directory_given_through_a_link_is_followed_because_the_person_named_it() {
        let dir = scratch();
        dir.make_dir("real").expect("made");
        dir.write("real/layer.yaml", "name: x\n").expect("written");
        dir.make_link("alias", "real").expect("linked");
        let opened = ReadOnlyDir::open(&dir.join("alias")).expect("opened");
        assert_eq!(
            opened.read_file(&path("layer.yaml"), CAP).expect("read"),
            b"name: x\n"
        );
    }

    #[test]
    fn a_directory_that_does_not_exist_or_is_a_file_is_missing() {
        let dir = scratch();
        dir.write("file", "x").expect("written");
        assert!(matches!(
            ReadOnlyDir::open(&dir.join("nowhere")),
            Err(ReadError::Missing)
        ));
        assert!(matches!(
            ReadOnlyDir::open(&dir.join("file")),
            Err(ReadError::Missing)
        ));
    }
}
