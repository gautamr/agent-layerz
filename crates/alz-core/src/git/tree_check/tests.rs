use std::collections::BTreeMap;

use super::{EntryDifference, verify_entry};
use crate::git::tree::TreeListing;
use crate::git::tree_entry::TreeEntry;
use crate::git::{BlobId, ObjectFormat};
use crate::layer::{GitFileFacts, LayerScan};
use crate::lock::ObjectId;
use crate::path::SafeRelPath;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

fn blob(text: &str) -> ObjectId {
    BlobId::of(ObjectFormat::Sha1, text.as_bytes()).id()
}

fn listing(files: &[(&str, &str, bool)]) -> TreeListing {
    let output: Vec<u8> = files
        .iter()
        .flat_map(|(name, text, executable)| {
            let mode = if *executable { "100755" } else { "100644" };
            format!("{mode} blob {}\t{name}\0", blob(text)).into_bytes()
        })
        .collect();
    TreeListing::from_entries(&TreeEntry::parse_all(&output).expect("output")).expect("a listing")
}

fn scan(files: &[(&str, &str, bool)]) -> LayerScan {
    files
        .iter()
        .fold(LayerScan::new(), |scan, (name, text, executable)| {
            scan.with_git_facts(GitFileFacts {
                path: path(name),
                executable: *executable,
                blob: blob(text),
            })
        })
}

const TREE: &[(&str, &str, bool)] = &[("a.md", "a", false), ("bin/run", "run", true)];

fn check(tree: &[(&str, &str, bool)], seen: &LayerScan) -> Result<(), EntryDifference> {
    verify_entry(&listing(tree), seen, &BTreeMap::new())
}

#[test]
fn an_entry_with_the_trees_files_matches() {
    assert_eq!(check(TREE, &scan(TREE)), Ok(()));
}

#[test]
fn each_kind_of_difference_is_found() {
    assert_eq!(
        check(TREE, &scan(&[("a.md", "a", false)])),
        Err(EntryDifference::Missing)
    );
    assert_eq!(
        check(TREE, &scan(&[TREE[0], TREE[1], ("b.md", "b", false)])),
        Err(EntryDifference::Extra)
    );
    assert_eq!(
        check(TREE, &scan(&[("a.md", "tampered", false), TREE[1]])),
        Err(EntryDifference::Changed)
    );
    assert_eq!(
        check(TREE, &scan(&[TREE[0], ("bin/run", "run", false)])),
        Err(EntryDifference::Mode)
    );
    assert_eq!(
        check(TREE, &scan(TREE).with_other_entry()),
        Err(EntryDifference::Link)
    );
}

#[test]
fn names_are_compared_in_nfc() {
    let tree = [("\u{e9}.md", "a", false)];
    assert_eq!(check(&tree, &scan(&[("e\u{301}.md", "a", false)])), Ok(()));
}

#[test]
fn env_files_are_checked_by_name_only() {
    let tree = [("a.md", "a", false), (".env.dev", "secret", false)];
    let seen = scan(&[tree[0]]).with_env_file(path(".env.dev"));
    assert_eq!(check(&tree, &seen), Ok(()));
    assert_eq!(check(&tree[..1], &seen), Err(EntryDifference::Extra));
    assert_eq!(
        check(&tree, &scan(&[tree[0]])),
        Err(EntryDifference::Missing)
    );
}

#[test]
fn names_the_walk_skips_are_ignored_unless_the_tree_has_them() {
    let tree = [("a.md", "a", false), ("node_modules/x.js", "x", false)];
    let listed = listing(&tree);
    let seen = scan(&[tree[0]]);
    let none = BTreeMap::new();
    assert_eq!(
        verify_entry(&listed, &seen, &none),
        Err(EntryDifference::Missing)
    );
    let found = BTreeMap::from([(path("node_modules/x.js"), blob("x"))]);
    assert_eq!(verify_entry(&listed, &seen, &found), Ok(()));
    let altered = BTreeMap::from([(path("node_modules/x.js"), blob("y"))]);
    assert_eq!(
        verify_entry(&listed, &seen, &altered),
        Err(EntryDifference::Changed)
    );
    // A skipped name the tree does not have is not looked at: the walk never lists it.
    assert_eq!(check(&tree[..1], &seen), Ok(()));
}
