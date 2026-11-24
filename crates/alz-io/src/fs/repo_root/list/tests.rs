//! The walk of the whole repository lists each folder once.

use std::collections::BTreeSet;
use std::fs;
use std::os::fd::OwnedFd;

use alz_test_support::ScratchDir;
use proptest::prelude::*;
use rustix::fs::{Mode, OFlags};

use super::walk;

fn open_root(path: &std::path::Path) -> OwnedFd {
    rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .expect("the folder opens")
}

/// Every `(folder, name)` pair the walk reports, in the order it reports them.
fn visited(root: &std::path::Path) -> Vec<(String, String)> {
    let mut seen = Vec::new();
    walk(&open_root(root), |folder, name, _| {
        seen.push((
            String::from_utf8_lossy(folder).into_owned(),
            name.to_string_lossy().into_owned(),
        ));
    })
    .expect("the walk succeeds");
    seen
}

/// A folder path of one to four segments over a small alphabet, so that folders share parents.
fn folder_path() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[abc]", 1..=4)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Whatever the shape of the tree, every entry is reported exactly once: a folder listed twice
    /// would report its entries twice, and a folder never listed would report none.
    #[test]
    fn each_folder_is_listed_once_and_every_entry_is_seen(
        folders in prop::collection::vec(folder_path(), 0..12),
        files in prop::collection::vec((folder_path(), "[xyz]\\.md"), 0..12),
    ) {
        let scratch = ScratchDir::new().expect("a scratch directory");
        let base = fs::canonicalize(scratch.path()).expect("a real path");
        let created: BTreeSet<(String, String)> = folders
            .iter()
            .map(|segments| (segments.as_slice(), None))
            .chain(files.iter().map(|(segments, name)| (segments.as_slice(), Some(name.as_str()))))
            .flat_map(|(segments, file)| {
                let folder = segments.join("/");
                fs::create_dir_all(base.join(&folder)).expect("made");
                if let Some(file) = file {
                    fs::write(base.join(&folder).join(file), "x").expect("written");
                }
                (0..segments.len())
                    .map(|depth| {
                        (
                            segments.get(..depth).unwrap_or_default().join("/"),
                            segments.get(depth).cloned().unwrap_or_default(),
                        )
                    })
                    .chain(file.map(|file| (folder.clone(), file.to_owned())))
                    .collect::<Vec<_>>()
            })
            .collect();
        let seen = visited(&base);
        let distinct: BTreeSet<(String, String)> = seen.iter().cloned().collect();
        prop_assert_eq!(seen.len(), distinct.len(), "an entry was reported twice");
        prop_assert_eq!(distinct, created);
    }
}

#[test]
fn a_folder_that_is_not_walked_is_reported_and_not_entered() {
    let scratch = ScratchDir::new().expect("a scratch directory");
    let base = fs::canonicalize(scratch.path()).expect("a real path");
    fs::create_dir_all(base.join(".git/hooks")).expect("made");
    fs::create_dir_all(base.join("node_modules/pkg")).expect("made");
    fs::create_dir_all(base.join("src")).expect("made");
    let seen = visited(&base);
    let names: BTreeSet<&str> = seen.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(
        names,
        BTreeSet::from([".git", "node_modules", "src"]),
        "only the root's own entries are listed"
    );
}
