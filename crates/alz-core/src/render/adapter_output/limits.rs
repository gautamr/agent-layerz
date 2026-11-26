//! The limits on an adapter's output (spec §8.0 **Output checks.**): 10000 files, 16 MiB for a
//! file and 256 MiB in all.

use crate::diagnostic::Message;

/// What an adapter's output may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Limits {
    files: usize,
    file_bytes: usize,
    total_bytes: usize,
}

impl Limits {
    /// The limits of the spec.
    pub(super) const SPEC: Self = Self::new(10_000, 16 * 1024 * 1024, 256 * 1024 * 1024);

    /// Limits of `files` files, `file_bytes` bytes for a file and `total_bytes` bytes in all.
    pub(super) const fn new(files: usize, file_bytes: usize, total_bytes: usize) -> Self {
        Self {
            files,
            file_bytes,
            total_bytes,
        }
    }

    /// Whether `count` files are within the limit.
    pub(super) fn check_count(&self, count: usize) -> Result<(), Message> {
        within(count, self.files, || {
            Message::new("it returned ")
                .then(count)
                .then(" files, more than the limit of ")
                .then(self.files)
        })
    }

    /// Whether a file of `bytes` bytes is within the limit.
    pub(super) fn check_file(&self, bytes: usize) -> Result<(), Message> {
        within(bytes, self.file_bytes, || {
            Message::new("it holds ")
                .then(bytes)
                .then(" bytes, more than the limit of ")
                .then(self.file_bytes)
                .then(" bytes for a file")
        })
    }

    /// Whether files of the sizes given are within the limit in all.
    pub(super) fn check_total(&self, sizes: impl Iterator<Item = usize>) -> Result<(), Message> {
        let total = sizes.fold(0_usize, usize::saturating_add);
        within(total, self.total_bytes, || {
            Message::new("the files hold ")
                .then(total)
                .then(" bytes in all, more than the limit of ")
                .then(self.total_bytes)
                .then(" bytes")
        })
    }
}

/// `Ok` when `value` is at most `limit`, and otherwise the error that `message` makes.
fn within(value: usize, limit: usize, message: impl FnOnce() -> Message) -> Result<(), Message> {
    if value > limit {
        Err(message())
    } else {
        Ok(())
    }
}
