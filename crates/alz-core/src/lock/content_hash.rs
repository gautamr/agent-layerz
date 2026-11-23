//! The content hash of a layer and the hashes of generated files (spec §5.3 "What the content hash
//! covers" and "Byte layout"; decision-34). Everything here is pure: it works over the paths and
//! bytes that a walk of the layer supplies, and never reads a file.

use std::collections::BTreeMap;
use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lock::hex;
use crate::path::SafeRelPath;

/// The prefix of a hash in `layer.lock`.
const PREFIX: &str = "sha256:";

/// Text that is not a SHA-256 digest as the lock writes it: `sha256:` and 64 lowercase
/// hexadecimal characters. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a sha256 hash: 'sha256:' and 64 lowercase hexadecimal characters")]
pub struct DigestError;

impl DigestError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be 'sha256:' and 64 lowercase hexadecimal characters"
    }
}

/// The SHA-256 of the bytes of one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileDigest([u8; 32]);

impl FileDigest {
    /// The digest of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The digest as 64 lowercase hexadecimal characters, which is how the byte layout of the
    /// content hash writes it.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

/// The hash of a generated file, as `layer.lock` stores it in `files`: `sha256:` and the 64
/// lowercase hexadecimal characters of the digest of the file's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileHash(FileDigest);

impl FileHash {
    /// The hash of the file whose bytes are `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(FileDigest::of(bytes))
    }
}

impl From<FileDigest> for FileHash {
    fn from(digest: FileDigest) -> Self {
        Self(digest)
    }
}

impl TryFrom<&str> for FileHash {
    type Error = DigestError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        parse(text).map(|bytes| Self(FileDigest(bytes)))
    }
}

impl fmt::Display for FileHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{PREFIX}{}", self.0.to_hex())
    }
}

fn parse(text: &str) -> Result<[u8; 32], DigestError> {
    text.strip_prefix(PREFIX)
        .and_then(hex::decode::<32>)
        .ok_or(DigestError)
}

/// The content hash of a layer, `sha256:` and 64 lowercase hexadecimal characters, as the lock
/// stores it in a layer's `contentHash`; the check compares that text exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// The content hash of `files` (spec §5.3 "Byte layout"): the SHA-256 of one UTF-8 text made
    /// of a record per file in name order, the path, a NUL, the lowercase hexadecimal digest of
    /// the file's bytes and a line feed, joined with nothing between them.
    #[must_use]
    pub fn of(files: &LayerFiles) -> Self {
        let text = files
            .0
            .iter()
            .map(|(path, digest)| format!("{path}\0{}\n", digest.to_hex()))
            .collect::<Vec<_>>()
            .concat();
        Self(Sha256::digest(text.as_bytes()).into())
    }
}

impl TryFrom<&str> for ContentHash {
    type Error = DigestError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        parse(text).map(Self)
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{PREFIX}{}", hex::encode(&self.0))
    }
}

/// The files of a layer that its content hash covers, each with the digest of its bytes. The
/// paths are in a map, so name order (spec §6) holds by construction: a [`SafeRelPath`] is one
/// string and orders by its UTF-8 bytes, which is code point order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerFiles(BTreeMap<SafeRelPath, FileDigest>);

impl LayerFiles {
    /// The number of files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the layer has no hashed file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The files and their digests in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&SafeRelPath, &FileDigest)> + '_ {
        self.0.iter()
    }

    /// The files of a layer from each path with the bytes of its file, so that a walk hands over
    /// content and nothing else. A path the hash does not cover (see [`is_hashed`]) is dropped.
    pub fn from_contents<'a>(files: impl IntoIterator<Item = (SafeRelPath, &'a [u8])>) -> Self {
        files
            .into_iter()
            .map(|(path, bytes)| (path, FileDigest::of(bytes)))
            .collect()
    }
}

impl FromIterator<(SafeRelPath, FileDigest)> for LayerFiles {
    /// A path the hash does not cover (see [`is_hashed`]) is dropped, so a `.env` file cannot
    /// enter the hash even when a caller lists it. A path that comes twice keeps the last digest.
    fn from_iter<I: IntoIterator<Item = (SafeRelPath, FileDigest)>>(files: I) -> Self {
        Self(
            files
                .into_iter()
                .filter(|(path, _)| is_hashed(path))
                .collect(),
        )
    }
}

/// Whether a name is one the walk of a layer never enters and the hash never covers: `.git`,
/// `.DS_Store` and `node_modules` (spec §5.3).
#[must_use]
pub fn is_skipped_name(name: &str) -> bool {
    matches!(name, ".git" | ".DS_Store" | "node_modules")
}

/// Whether `path` is a `.env` file: its own name, the last segment, starts with `.env` (spec §5.3
/// "`.env` files"). `.env`, `.env.dev` and `.envrc` are; `env.md`, and `x` inside a folder named
/// `.env`, are not. Such a file is never read and not hashed.
#[must_use]
pub fn is_env_file(path: &SafeRelPath) -> bool {
    path.segments()
        .last()
        .is_some_and(|name| name.starts_with(".env"))
}

/// Whether the content hash covers `path`: it is no `.env` file and has no skipped name in it.
#[must_use]
pub fn is_hashed(path: &SafeRelPath) -> bool {
    !is_env_file(path) && !path.segments().any(is_skipped_name)
}

#[cfg(test)]
mod tests;
