use proptest::prelude::{prop_assert_eq, proptest};

use super::LayerContents;
use crate::lock::{ContentHash, LayerFiles};
use crate::path::SafeRelPath;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

#[test]
fn the_digests_of_the_contents_are_the_files_the_hash_covers() {
    let contents: LayerContents = [
        (path("b.md"), b"b".to_vec()),
        (path("a/c.md"), b"c".to_vec()),
        (path("a.md"), Vec::new()),
    ]
    .into_iter()
    .collect();
    let expected = LayerFiles::from_contents([
        (path("b.md"), b"b".as_slice()),
        (path("a/c.md"), b"c".as_slice()),
        (path("a.md"), b"".as_slice()),
    ]);
    assert_eq!(contents.files(), expected);
    assert_eq!(
        ContentHash::of(&contents.files()),
        ContentHash::of(&expected)
    );
    let names: Vec<&str> = contents.paths().map(SafeRelPath::as_str).collect();
    assert_eq!(names, ["a.md", "a/c.md", "b.md"]);
    assert_eq!(contents.get(&path("b.md")), Some(b"b".as_slice()));
    assert_eq!(contents.get(&path("none")), None);
}

#[test]
fn a_path_the_hash_does_not_cover_is_not_held() {
    let contents: LayerContents = [
        (path(".env"), b"SECRET=1".to_vec()),
        (path("x/.env.dev"), b"SECRET=2".to_vec()),
        (path("node_modules/a"), b"a".to_vec()),
        (path("ok.md"), b"ok".to_vec()),
    ]
    .into_iter()
    .collect();
    assert_eq!(contents.len(), 1);
    assert_eq!(contents.files().len(), 1);
}

#[test]
fn debug_shows_sizes_and_never_bytes() {
    let contents: LayerContents = [(path("a.md"), b"secret text".to_vec())]
        .into_iter()
        .collect();
    let shown = format!("{contents:?}");
    assert!(!shown.contains("secret"), "{shown}");
    assert!(shown.contains("a.md"), "{shown}");
}

proptest! {
    #[test]
    fn the_digests_of_any_contents_equal_the_files_made_from_the_same_bytes(
        files in proptest::collection::btree_map("[a-z]{1,3}(/[a-z]{1,3}){0,2}", proptest::collection::vec(proptest::num::u8::ANY, 0..20), 0..8)
    ) {
        let contents: LayerContents = files
            .iter()
            .map(|(name, bytes)| (path(name), bytes.clone()))
            .collect();
        let expected = LayerFiles::from_contents(
            files.iter().map(|(name, bytes)| (path(name), bytes.as_slice())),
        );
        prop_assert_eq!(contents.files(), expected);
    }
}
