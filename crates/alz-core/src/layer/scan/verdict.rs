//! The order of checks of spec §5.3 "File names", applied to a [`LayerScan`].

use std::collections::BTreeSet;

use crate::diagnostic::Diagnostic;
use crate::layer::contents::LayerContents;
use crate::layer::scan::{LayerScan, NameProblem, Purpose};
use crate::lock::LayerFiles;
use crate::path::SafeRelPath;
use crate::stack::resolution_failure::file_name_not_utf8;
use crate::untrusted_text::Stripped;

/// Why a layer is refused, in the order the checks are made, so the derived order is the order of
/// precedence: the first that fails decides (§5.3). The content hash check lies between
/// [`LayerRefusal::Unreadable`] and [`LayerRefusal::EnvFile`]; it needs the lock, so the caller
/// makes it on the [`ReadLayer`] that the verdict gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerRefusal {
    /// More than 2000 entries (account resolution only).
    TooManyEntries,
    /// A name that is not valid Unicode.
    NameNotUtf8,
    /// A path that is no safe relative path.
    NameNotSafe,
    /// A folder or file that cannot be read, a file of more than 16 MiB, or more than 16 MiB of
    /// files in all (account resolution only for the last).
    Unreadable,
    /// A `.env` file.
    EnvFile,
}

impl LayerRefusal {
    /// How account resolution names the cache miss this refusal is (§5.3, §7.13).
    #[must_use]
    pub const fn miss(self) -> &'static str {
        match self {
            Self::TooManyEntries | Self::Unreadable => "unreadable",
            Self::NameNotUtf8 | Self::NameNotSafe | Self::EnvFile => "layer-invalid",
        }
    }

    /// The error `layer` the build reports for a name that is not valid UTF-8 (§5.3): it names
    /// the layer by its source and never the name. `None` for every other refusal, whose build
    /// message is not specified.
    #[must_use]
    pub fn file_name_diagnostic(self, source: Stripped) -> Option<Diagnostic> {
        match self {
            Self::NameNotUtf8 => Some(file_name_not_utf8(source)),
            Self::TooManyEntries | Self::NameNotSafe | Self::Unreadable | Self::EnvFile => None,
        }
    }
}

impl From<NameProblem> for LayerRefusal {
    fn from(problem: NameProblem) -> Self {
        match problem {
            NameProblem::NotUtf8 => Self::NameNotUtf8,
            NameProblem::NotSafe => Self::NameNotSafe,
        }
    }
}

/// A scan that passed the checks before the content hash: the files the hash covers with the
/// bytes that were read, which are only names the verdict has validated, and the `.env` files
/// that were listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLayer {
    contents: LayerContents,
    files: LayerFiles,
    env_files: Vec<SafeRelPath>,
}

impl ReadLayer {
    /// The layer a scan that passed the checks holds.
    fn of(scan: LayerScan) -> Self {
        let contents: LayerContents = scan.files.into_iter().collect();
        Self {
            files: contents.files(),
            contents,
            env_files: scan
                .env_files
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    /// The files the content hash covers, with the digest of the bytes the walk read.
    #[must_use]
    pub const fn files(&self) -> &LayerFiles {
        &self.files
    }

    /// The files the content hash covers, with the bytes the walk read.
    #[must_use]
    pub const fn contents(&self) -> &LayerContents {
        &self.contents
    }

    /// The `.env` files, in code point order. None was opened.
    #[must_use]
    pub fn env_files(&self) -> &[SafeRelPath] {
        &self.env_files
    }

    /// The last check of account resolution, after the content hash: a layer with a `.env` file
    /// is refused ([`LayerRefusal::EnvFile`]). It gives the files otherwise.
    ///
    /// # Errors
    ///
    /// [`LayerRefusal::EnvFile`] when the layer holds a `.env` file.
    pub fn without_env_files(self) -> Result<LayerFiles, LayerRefusal> {
        if self.env_files.is_empty() {
            Ok(self.files)
        } else {
            Err(LayerRefusal::EnvFile)
        }
    }

    /// The files with their bytes and the `.env` files, for the build, which reports each `.env`
    /// file. The digests are [`LayerContents::files`].
    #[must_use]
    pub fn into_parts(self) -> (LayerContents, Vec<SafeRelPath>) {
        (self.contents, self.env_files)
    }
}

/// Applies the order of checks to `scan` for `purpose` and gives the layer, or the first refusal
/// (§5.3 "File names"):
///
/// 1. too many entries (`unreadable`; account resolution only);
/// 2. a name that is not valid Unicode (`layer-invalid`), then a path that is not safe;
/// 3. a file that could not be read, a file of more than 16 MiB (`unreadable`; the build too), or
///    reading past 16 MiB in all (`unreadable`; account resolution only).
///
/// The content hash and the `.env` check follow on the result. In the build nothing is limited,
/// so the name check comes first and the budgets in all are not applied.
///
/// # Errors
///
/// The first of those checks that fails, as a [`LayerRefusal`].
pub fn scan_verdict(purpose: Purpose, scan: LayerScan) -> Result<ReadLayer, LayerRefusal> {
    refusal(purpose, &scan).map_or_else(|| Ok(ReadLayer::of(scan)), Err)
}

fn refusal(purpose: Purpose, scan: &LayerScan) -> Option<LayerRefusal> {
    scan.is_past_entry_limit(purpose)
        .then_some(LayerRefusal::TooManyEntries)
        .or_else(|| scan.name_problem.map(LayerRefusal::from))
        .or_else(|| {
            (scan.unreadable || scan.is_past_byte_limit(purpose))
                .then_some(LayerRefusal::Unreadable)
        })
}
