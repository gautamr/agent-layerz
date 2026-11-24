use proptest::collection::btree_map;
use proptest::prelude::{any, prop_assert_eq, proptest};

use crate::lock::{
    ContentHash, DigestError, FileDigest, FileHash, LayerFiles, is_env_file, is_hashed,
    is_skipped_name,
};
use crate::path::SafeRelPath;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

/// The content hash of a layer whose files are `(path, content)`, listed in any order.
fn hash_of(files: &[(&str, &str)]) -> String {
    ContentHash::of(&LayerFiles::from_contents(
        files
            .iter()
            .map(|(name, content)| (path(name), content.as_bytes())),
    ))
    .to_string()
}

#[test]
fn sha256_matches_the_standard_vectors() {
    assert_eq!(
        FileDigest::of(b"").to_hex(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        FileDigest::of(b"abc").to_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// Vectors computed by the formula of spec §6 (R1.2) on real directories with the same files,
/// recorded in TASK-55.3.7's notes. The formula orders by code
/// point, so the layer with a character above U+FFFF and one in U+E000-U+FFFF is the case where
/// an order by UTF-16 unit would differ.
#[test]
fn content_hash_matches_the_test_side_formula_on_the_vectors() {
    type Vector<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str);
    let vectors: [Vector<'_>; 5] = [
        (
            "an empty layer hashes the empty text",
            &[],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "one file",
            &[("a.md", "hello\n")],
            "27bef2562abfb2075cb78619ffbb0f4e21948f13622d28f9400549e5b875d4b1",
        ),
        (
            "nested paths, listed out of order: 10 before 9, a-b.md before a/b.md",
            &[
                ("b.md", "b"),
                ("a/c/d", "d"),
                ("9", "nine"),
                ("a/b.md", "ab"),
                ("10", "ten"),
                ("a-b.md", "a-b"),
            ],
            "c06b09ad7046b7d33857803f2c3876c5624747c335003d9d4fda907f7730393b",
        ),
        (
            "a folder named .env holds a file that is not a .env file",
            &[(".env/x", "inside"), ("a.md", "a")],
            "731b8e6607d2568acdbc7e191a52c728ac0e8a6ed432395b6e44da25eb53f00a",
        ),
        (
            "code point order: U+E000 and U+FF5E sort before U+1F600, which UTF-16 order reverses",
            &[
                ("\u{1F600}", "astral"),
                ("\u{ff5e}", "bmp"),
                ("\u{e9}", "e"),
                ("z", "z"),
                ("\u{e000}", "pua"),
            ],
            "c87a5f036a5a137c52c521c50c0ee1bd22d96c3ed34b77667842cf7b6e789ba9",
        ),
    ];
    for (name, files, hex) in vectors {
        assert_eq!(hash_of(files), format!("sha256:{hex}"), "{name}");
    }
}

#[test]
fn files_the_hash_does_not_cover_never_enter_it() {
    // `.env` files, `.DS_Store` and `node_modules` are in the walk's way and in this list on
    // purpose: the vector is the one the spec's formula gives without them.
    assert_eq!(
        hash_of(&[
            ("x.md", "x"),
            (".env.dev", "SECRET"),
            (".envrc", "SECRET"),
            ("env.md", "e"),
            (".env/x", "inside"),
            (".DS_Store", "d"),
            ("sub/.DS_Store", "d"),
            ("node_modules/p/i.js", "n"),
            ("sub/node_modules/q", "n"),
        ]),
        "sha256:83bee2f69f8a476042a6a0f0d5114c3abbc1872bd99c714d9b4718c88d57deba"
    );
}

#[test]
fn the_byte_layout_is_path_nul_hex_digest_line_feed() {
    let digest = FileDigest::of(b"hello\n");
    let record = format!("a.md\0{}\n", digest.to_hex());
    assert_eq!(
        hash_of(&[("a.md", "hello\n")]),
        format!("sha256:{}", FileDigest::of(record.as_bytes()).to_hex())
    );
}

#[test]
fn env_files_are_the_files_whose_own_name_starts_with_dot_env() {
    for env in [".env", ".env.dev", ".envrc", "a/.env", "a/b/.env.local"] {
        assert!(is_env_file(&path(env)), "{env}");
        assert!(!is_hashed(&path(env)), "{env}");
    }
    for other in ["env.md", ".env/x", "a/.env/x", "a.env", "x.envrc"] {
        assert!(!is_env_file(&path(other)), "{other}");
        assert!(is_hashed(&path(other)), "{other}");
    }
}

#[test]
fn the_skipped_names_are_git_ds_store_and_node_modules() {
    for name in [".git", ".DS_Store", "node_modules"] {
        assert!(is_skipped_name(name), "{name}");
    }
    for name in [".github", "node_modules2", ".ds_store", "git"] {
        assert!(!is_skipped_name(name), "{name}");
    }
    assert!(!is_hashed(&path("a/node_modules/b")));
    assert!(!is_hashed(&path("node_modules")));
}

#[test]
fn a_hash_is_sha256_and_sixty_four_lowercase_hex_characters_and_nothing_else() {
    let good = format!("sha256:{}", "ab".repeat(32));
    assert_eq!(
        ContentHash::try_from(good.as_str()).map(|hash| hash.to_string()),
        Ok(good.clone())
    );
    assert_eq!(
        FileHash::try_from(good.as_str()).map(|hash| hash.to_string()),
        Ok(good)
    );
    let bad = [
        String::new(),
        "ab".repeat(32),
        format!("sha256:{}", "ab".repeat(31)),
        format!("sha256:{}a", "ab".repeat(32)),
        format!("sha256:{}", "AB".repeat(32)),
        format!("SHA256:{}", "ab".repeat(32)),
        format!("sha1:{}", "ab".repeat(32)),
        format!("sha256: {}", "ab".repeat(32)),
        format!("sha256:{}g", "ab".repeat(31) + "a"),
    ];
    for text in &bad {
        assert_eq!(
            ContentHash::try_from(text.as_str()),
            Err(DigestError),
            "{text:?}"
        );
        assert_eq!(
            FileHash::try_from(text.as_str()),
            Err(DigestError),
            "{text:?}"
        );
    }
}

proptest! {
    #[test]
    fn a_content_hash_prints_back_as_it_was_read(bytes in any::<[u8; 32]>()) {
        let text = format!("sha256:{}", super::hex::encode(&bytes));
        prop_assert_eq!(
            ContentHash::try_from(text.as_str()).map(|hash| hash.to_string()),
            Ok(text.clone())
        );
        prop_assert_eq!(
            FileHash::try_from(text.as_str()).map(|hash| hash.to_string()),
            Ok(text)
        );
    }

    /// The hash is a function of the set of files, not of the order a walk lists them in.
    #[test]
    fn the_order_a_walk_lists_files_in_does_not_change_the_hash(
        files in btree_map("[a-z0-9./-]{1,8}", "[a-z]{0,4}", 0..8),
    ) {
        let entries: Vec<(SafeRelPath, FileDigest)> = files
            .iter()
            .filter_map(|(name, content)| {
                SafeRelPath::try_from(name.as_str())
                    .ok()
                    .map(|safe| (safe, FileDigest::of(content.as_bytes())))
            })
            .collect();
        let forward: LayerFiles = entries.iter().cloned().collect();
        let backward: LayerFiles = entries.iter().rev().cloned().collect();
        prop_assert_eq!(ContentHash::of(&forward), ContentHash::of(&backward));
    }
}
