//! The two files every command that reads a repository's stack starts from, `layer.yaml` and
//! `layer.lock`, read through a handle on the working directory: no link is followed and neither
//! file is larger than 1 MiB.

use std::path::Path;

use alz_core::path::{SafeRelPath, SafeRelPathError};
use alz_core::yaml::MAX_BYTES;
use alz_core::{Diagnostic, Stripped};
use alz_io::{ReadError, ReadOnlyDir};

/// The directory the repository files are read from when no other is given.
pub const WORKING_DIRECTORY: &str = ".";

/// The regular file `name` of `directory`, at most 1 MiB, never reached through a link.
pub fn read_in(directory: &Path, name: &str) -> Result<Vec<u8>, ReadError> {
    let path = SafeRelPath::try_from(name).map_err(|SafeRelPathError| ReadError::Missing)?;
    ReadOnlyDir::open(directory)?.read_file(&path, MAX_BYTES)
}

/// The `layer.yaml` of `directory`, or the diagnostic that says why it cannot be read.
pub fn read_layer_yaml(directory: &Path, source: &Stripped) -> Result<Vec<u8>, Diagnostic> {
    read_in(directory, "layer.yaml").map_err(|error| error.into_layer_yaml_diagnostic(source))
}

/// The `layer.lock` of `directory`: `None` when there is none.
pub fn read_lock(directory: &Path) -> Result<Option<Vec<u8>>, Diagnostic> {
    match read_in(directory, "layer.lock") {
        Ok(bytes) => Ok(Some(bytes)),
        Err(ReadError::Missing) => Ok(None),
        Err(error) => Err(error.into_lock_diagnostic()),
    }
}
