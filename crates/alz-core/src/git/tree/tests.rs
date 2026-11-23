use proptest::prelude::{any, prop_assert, proptest};

use super::{ListingRefusal, TreeListing, nfc};
use crate::git::tree_entry::TreeEntry;

const A: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";
const B: &str = "0123456789abcdef0123456789abcdef01234567";

fn record(mode: &str, kind: &str, id: &str, path: &[u8]) -> Vec<u8> {
    [format!("{mode} {kind} {id}\t").as_bytes(), path, b"\0"].concat()
}

fn entries(records: &[Vec<u8>]) -> Vec<TreeEntry> {
    TreeEntry::parse_all(&records.concat()).expect("readable output")
}

fn listing(records: &[Vec<u8>]) -> Result<TreeListing, ListingRefusal> {
    TreeListing::from_entries(&entries(records))
}

fn blob(path: &str) -> Vec<u8> {
    record("100644", "blob", A, path.as_bytes())
}

#[test]
fn files_are_listed_with_their_blob_and_executable_bit_and_links_are_skipped() {
    let tree = listing(&[
        blob("a.md"),
        record("100755", "blob", B, b"bin/run"),
        record("120000", "blob", A, b"CLAUDE.md"),
        record("160000", "commit", A, b"vendor"),
    ])
    .expect("a listing");
    let files: Vec<_> = tree
        .files()
        .map(|(path, file)| (path.as_str().to_owned(), file.executable))
        .collect();
    assert_eq!(
        files,
        [("a.md".to_owned(), false), ("bin/run".to_owned(), true)]
    );
    assert_eq!(tree.len(), 2);
    assert_eq!(tree.blob_ids().len(), 2);
}

#[test]
fn a_path_that_is_not_utf8_is_refused_whatever_else_is_wrong() {
    assert_eq!(
        listing(&[record("100644", "blob", A, b"a\xff.md")]),
        Err(ListingRefusal::NotUtf8)
    );
    assert_eq!(
        listing(&[
            blob("a\\b"),
            record("100644", "blob", A, b"d/\xfe"),
            blob("x"),
            blob("X")
        ]),
        Err(ListingRefusal::NotUtf8)
    );
}

#[test]
fn a_link_or_a_submodule_with_a_bad_path_is_refused_too() {
    assert_eq!(
        listing(&[record("120000", "blob", A, b"a\xff")]),
        Err(ListingRefusal::NotUtf8)
    );
    assert_eq!(
        listing(&[record("160000", "commit", A, b".git")]),
        Err(ListingRefusal::NotSafe)
    );
}

#[test]
fn a_path_that_is_not_safe_is_refused() {
    for path in [
        "a\\b.md",
        "a/../b",
        "a/./b",
        ".git/config",
        ".GIT/config",
        "x/.Git",
        ".git./x",
        ".git /x",
        "a//b",
        "",
    ] {
        assert_eq!(
            listing(&[blob(path)]),
            Err(ListingRefusal::NotSafe),
            "{path:?}"
        );
    }
}

#[test]
fn two_paths_equal_after_nfc_and_case_folding_are_refused() {
    let nfd = "e\u{301}.md";
    for pair in [
        ("a.md", "A.md"),
        ("\u{e9}.md", nfd),
        ("dir/x", "DIR/y"),
        ("Dir", "dir/x"),
        ("\u{c9}.md", nfd),
    ] {
        assert_eq!(
            listing(&[blob(pair.0), blob(pair.1)]),
            Err(ListingRefusal::Collision),
            "{pair:?}"
        );
    }
    assert!(listing(&[blob("a.md"), blob("b.md"), blob("a/b.md")]).is_ok());
}

#[test]
fn a_link_does_not_collide_with_a_file_because_it_is_not_written() {
    assert!(listing(&[blob("a.md"), record("120000", "blob", A, b"A.md")]).is_ok());
}

#[test]
fn the_files_under_names_the_walk_skips_are_told_apart() {
    let tree = listing(&[
        blob("a.md"),
        blob("node_modules/x/y.js"),
        blob("d/.DS_Store"),
    ])
    .expect("a listing");
    let skipped: Vec<_> = tree.skipped().map(|(path, _)| path.as_str()).collect();
    assert_eq!(skipped, ["d/.DS_Store", "node_modules/x/y.js"]);
}

proptest! {
    #[test]
    fn no_two_accepted_paths_are_equal_after_nfc_and_lowercasing(
        names in proptest::collection::vec("[a-cA-C\u{e9}\u{c9}/]{1,4}", 1..6)
    ) {
        let records: Vec<_> = names.iter().map(|name| blob(name)).collect();
        if let Ok(tree) = listing(&records) {
            let folded: Vec<String> = tree
                .files()
                .map(|(path, _)| nfc(path.as_str()).to_lowercase())
                .collect();
            let mut unique = folded.clone();
            unique.sort();
            unique.dedup();
            prop_assert!(unique.len() == folded.len());
        }
    }

    #[test]
    fn a_listing_never_holds_a_path_that_is_not_safe(bytes in any::<Vec<u8>>()) {
        // git ends a path with a NUL, so a path holds none.
        let bytes: Vec<u8> = bytes.into_iter().filter(|byte| *byte != 0).collect();
        let records = [record("100644", "blob", A, &bytes)];
        if let Ok(tree) = listing(&records) {
            prop_assert!(tree.files().all(|(path, _)| !path.as_str().contains('\\')));
        }
    }
}
