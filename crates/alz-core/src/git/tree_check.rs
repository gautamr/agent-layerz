//! The check of a `tree2@` entry against its git tree (spec §6 "A pinned `path:` layer"): the
//! facts the walk saw in the entry, compared with `git ls-tree -r`. Pure: nothing is read here.
//!
//! The entry is right when every file the tree lists is present with the blob id git computes
//! over its bytes and the executable bit of its mode, and nothing else is there: no extra file
//! and no link. Names are compared in NFC. Names the walk skips are ignored in the entry unless
//! the tree has them, in which case they are checked by their path.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::git::tree::{TreeListing, nfc};
use crate::layer::LayerScan;
use crate::lock::{ObjectId, is_env_file};
use crate::path::SafeRelPath;

/// How an entry differs from its tree. It names the kind of difference and never a name or
/// content, which the entry's own files make untrusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EntryDifference {
    /// A link, a pipe or another entry that is no file is in the entry.
    #[error("the entry holds a link or another entry that is not a file")]
    Link,
    /// A file the entry holds is not in the tree.
    #[error("the entry holds a file the tree does not")]
    Extra,
    /// A file of the tree is not in the entry.
    #[error("the entry lacks a file of the tree")]
    Missing,
    /// A file's bytes are not the tree's blob.
    #[error("a file differs from its blob")]
    Changed,
    /// A file's executable bit is not the tree's mode.
    #[error("a file's executable bit differs from its mode")]
    Mode,
}

/// What the entry holds for the hashed files, by NFC name: blob and executable bit.
fn observed(scan: &LayerScan) -> BTreeMap<String, (ObjectId, bool)> {
    scan.git_files()
        .iter()
        .map(|facts| (nfc(facts.path.as_str()), (facts.blob, facts.executable)))
        .collect()
}

fn names(paths: &[SafeRelPath]) -> BTreeSet<String> {
    paths.iter().map(|path| nfc(path.as_str())).collect()
}

fn first_difference<T: Ord>(seen: &BTreeSet<&T>, wanted: &BTreeSet<&T>) -> Option<EntryDifference> {
    if !seen.is_subset(wanted) {
        Some(EntryDifference::Extra)
    } else if !wanted.is_subset(seen) {
        Some(EntryDifference::Missing)
    } else {
        None
    }
}

/// Compares what the walk of an entry saw with the tree it was written from.
///
/// `scan` must have been made with the blob ids of its files. `skipped_found` holds, for each
/// path of [`TreeListing::skipped`] that the entry has, the blob id of its bytes.
///
/// # Errors
///
/// The first [`EntryDifference`], in this order: a link, an extra file, a missing file, a file
/// whose bytes differ, a file whose executable bit differs.
pub fn verify_entry(
    listing: &TreeListing,
    scan: &LayerScan,
    skipped_found: &BTreeMap<SafeRelPath, ObjectId>,
) -> Result<(), EntryDifference> {
    if scan.other_entries() > 0 {
        return Err(EntryDifference::Link);
    }
    let skipped: BTreeSet<&SafeRelPath> = listing.skipped().map(|(path, _)| path).collect();
    let walked = || listing.files().filter(|(path, _)| !skipped.contains(path));
    let wanted: BTreeMap<String, (ObjectId, bool)> = walked()
        .filter(|(path, _)| !is_env_file(path))
        .map(|(path, file)| (nfc(path.as_str()), (file.blob, file.executable)))
        .collect();
    let wanted_env: BTreeSet<String> = walked()
        .filter(|(path, _)| is_env_file(path))
        .map(|(path, _)| nfc(path.as_str()))
        .collect();
    let seen = observed(scan);
    let seen_env = names(scan.env_files());
    let seen_names: BTreeSet<&String> = seen.keys().collect();
    let wanted_names: BTreeSet<&String> = wanted.keys().collect();
    let seen_env_names: BTreeSet<&String> = seen_env.iter().collect();
    let wanted_env_names: BTreeSet<&String> = wanted_env.iter().collect();
    let skipped_missing = listing
        .skipped()
        .any(|(path, _)| !skipped_found.contains_key(path));
    if let Some(difference) = first_difference(&seen_names, &wanted_names)
        .or_else(|| first_difference(&seen_env_names, &wanted_env_names))
    {
        return Err(difference);
    }
    if skipped_missing {
        return Err(EntryDifference::Missing);
    }
    let skipped_changed = listing
        .skipped()
        .any(|(path, file)| skipped_found.get(path) != Some(&file.blob));
    let changed = wanted
        .iter()
        .any(|(name, (blob, _))| seen.get(name).map(|(found, _)| found) != Some(blob));
    if changed || skipped_changed {
        return Err(EntryDifference::Changed);
    }
    let mode_differs = wanted
        .iter()
        .any(|(name, (_, executable))| seen.get(name).map(|(_, found)| found) != Some(executable));
    if mode_differs {
        Err(EntryDifference::Mode)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
