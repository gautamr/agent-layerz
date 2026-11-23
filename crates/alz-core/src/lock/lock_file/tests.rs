use std::collections::BTreeMap;

use proptest::collection::{btree_map, vec};
use proptest::prelude::{Just, Strategy, any, prop_assert_eq, prop_oneof, proptest};

use crate::json::JsonErrorKind;
use crate::layer::{LayerName, Visibility};
use crate::lock::{ContentHash, FileHash, Lock, LockError, LockField, LockedLayer, ObjectId};
use crate::path::SafeRelPath;
use crate::stack::{GitRef, Source};

/// This repository's own lock, written by the TypeScript `alz`.
const REPOSITORY_LOCK: &[u8] = include_bytes!("../../../../../layer.lock");

const SHA: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";
const HASH: &str = "sha256:0b67587a87804b2532bac6eb70dc7f436e0abebb8bdf24e638cb801fc99fe737";

fn lock_with(layer: &str) -> String {
    format!(
        r#"{{"files":{{"AGENTS.md":"{HASH}"}},"layers":[{layer}],"lockVersion":1,"visibility":"public"}}"#
    )
}

fn layer_with(sha: &str, content_hash: &str) -> String {
    format!(
        r#"{{"contentHash":"{content_hash}","name":"x","rev":"v1","sha":"{sha}","source":"path:./x"}}"#
    )
}

fn prefix(text: &str, length: usize) -> String {
    text.chars().take(length).collect()
}

fn parse(text: &str) -> Result<Lock, LockError> {
    Lock::parse(text.as_bytes())
}

#[test]
fn this_repositorys_lock_parses_and_is_emitted_again_byte_for_byte() {
    let lock = Lock::parse(REPOSITORY_LOCK).expect("the repository's lock is valid");
    assert!(lock.layers.len() > 1);
    assert!(lock.files.len() > 1);
    assert_eq!(lock.emit().as_bytes(), REPOSITORY_LOCK);
}

#[test]
fn a_lock_with_every_source_form_is_emitted_again_as_it_was_read() {
    let text = format!(
        r#"{{
  "files": {{
    "a-b.md": "{HASH}",
    "a/b.md": "{HASH}"
  }},
  "layers": [
    {{
      "contentHash": "{HASH}",
      "name": "client.hub",
      "rev": "v2.1.0",
      "sha": "{SHA}",
      "source": "git:https://example.test/hub.git@v2.1.0"
    }},
    {{
      "contentHash": "{HASH}",
      "name": "team",
      "rev": "main",
      "sha": "{}",
      "source": "github:owner/repo@main"
    }},
    {{
      "contentHash": "{HASH}",
      "name": "here",
      "source": "path:./.layer"
    }}
  ],
  "lockVersion": 1,
  "visibility": "confidential"
}}
"#,
        "ab".repeat(32)
    );
    let lock = parse(&text).expect("a valid lock");
    assert_eq!(lock.visibility, Visibility::Confidential);
    assert_eq!(lock.layers.len(), 3);
    assert!(matches!(lock.layers[1].sha, Some(ObjectId::Sha256(_))));
    assert_eq!(lock.layers[2].sha, None);
    assert_eq!(lock.emit(), text);
}

#[test]
fn a_sha_that_is_not_an_object_id_is_refused_before_any_value_is_used() {
    let too_short = prefix(SHA, 39);
    let too_long = format!("{SHA}0");
    let upper = SHA.to_uppercase();
    for sha in [&too_short, &too_long, &upper, "", "../../x", "main"] {
        assert_eq!(
            parse(&lock_with(&layer_with(sha, HASH))),
            Err(LockError::Invalid(LockField::LayerSha)),
            "{sha:?}"
        );
    }
    let not_text = lock_with(&format!(
        r#"{{"contentHash":"{HASH}","name":"x","sha":1,"source":"path:./x"}}"#
    ));
    assert_eq!(
        parse(&not_text),
        Err(LockError::Invalid(LockField::LayerSha))
    );
}

#[test]
fn a_content_hash_that_is_not_sha256_and_64_lowercase_hex_is_refused() {
    let upper = HASH.to_uppercase().replace("SHA256", "sha256");
    let short = prefix(HASH, HASH.len() - 1);
    let bare: String = HASH.chars().skip("sha256:".len()).collect();
    for hash in [&upper, &short, &bare, "", "sha256:"] {
        assert_eq!(
            parse(&lock_with(&layer_with(SHA, hash))),
            Err(LockError::Invalid(LockField::LayerContentHash)),
            "{hash:?}"
        );
    }
}

#[test]
fn a_file_entry_that_is_not_a_safe_path_with_a_hash_is_refused() {
    for path in ["../x", "/x", "a/../b", ".git/config", "a\\\\b", ""] {
        let text = format!(
            r#"{{"files":{{"{path}":"{HASH}"}},"layers":[],"lockVersion":1,"visibility":"public"}}"#
        );
        assert_eq!(
            parse(&text),
            Err(LockError::Invalid(LockField::FilePath)),
            "{path:?}"
        );
    }
    let text = r#"{"files":{"a":"sha1:abc"},"layers":[],"lockVersion":1,"visibility":"public"}"#;
    assert_eq!(parse(text), Err(LockError::Invalid(LockField::FileHash)));
}

#[test]
fn a_duplicate_key_in_any_object_makes_the_lock_invalid() {
    let at_top =
        r#"{"files":{},"layers":[],"lockVersion":1,"lockVersion":1,"visibility":"public"}"#
            .to_owned();
    let in_layer = lock_with(&format!(
        r#"{{"contentHash":"{HASH}","name":"x","name":"y","source":"path:./x"}}"#
    ));
    let in_files = format!(
        r#"{{"files":{{"a":"{HASH}","a":"{HASH}"}},"layers":[],"lockVersion":1,"visibility":"public"}}"#
    );
    for text in [at_top, in_layer, in_files] {
        let Err(LockError::Json(error)) = parse(&text) else {
            panic!("a duplicate key must make the lock invalid: {text}");
        };
        assert!(matches!(error.kind(), JsonErrorKind::DuplicateKey(_)));
        let line = LockError::Json(error).into_diagnostic().to_string();
        assert!(line.starts_with("error [lock] layer.lock:1:"), "{line}");
        assert!(line.contains(": duplicate key '"), "{line}");
    }
}

#[test]
fn a_lock_version_other_than_one_is_refused() {
    let with = |version: &str| {
        format!(r#"{{"files":{{}},"layers":[],"lockVersion":{version},"visibility":"public"}}"#)
    };
    for version in ["2", "0", "\"1\"", "null", "true", "1.5", "[1]"] {
        assert_eq!(
            parse(&with(version)),
            Err(LockError::Invalid(LockField::LockVersion)),
            "{version}"
        );
    }
    assert!(parse(&with("1")).is_ok());
    let missing = r#"{"files":{},"layers":[],"visibility":"public"}"#;
    assert_eq!(
        parse(missing),
        Err(LockError::Invalid(LockField::LockVersion))
    );
}

#[test]
fn a_document_of_the_wrong_shape_is_refused_naming_the_field_and_no_value() {
    let cases = [
        ("[]", LockField::Document),
        ("null", LockField::Document),
        (r#"{"lockVersion":1}"#, LockField::Visibility),
        (
            r#"{"files":{},"layers":{},"lockVersion":1,"visibility":"public"}"#,
            LockField::Layers,
        ),
        (
            r#"{"files":[],"layers":[],"lockVersion":1,"visibility":"public"}"#,
            LockField::Files,
        ),
        (
            r#"{"files":{},"layers":[],"lockVersion":1,"visibility":"secret"}"#,
            LockField::Visibility,
        ),
        (
            r#"{"files":{},"layers":[1],"lockVersion":1,"visibility":"public"}"#,
            LockField::Layers,
        ),
    ];
    for (text, field) in cases {
        assert_eq!(parse(text), Err(LockError::Invalid(field)), "{text}");
    }
    let line = LockError::Invalid(LockField::Visibility)
        .into_diagnostic()
        .to_string();
    assert_eq!(
        line,
        "error [lock] layer.lock: the lock's visibility is missing or not valid; delete the lock and rebuild"
    );
    let bad_name = lock_with(&format!(
        r#"{{"contentHash":"{HASH}","name":"Not A Name","source":"path:./x"}}"#
    ));
    assert_eq!(
        parse(&bad_name),
        Err(LockError::Invalid(LockField::LayerName))
    );
    let bad_source = lock_with(&format!(
        r#"{{"contentHash":"{HASH}","name":"x","source":"https://example.test/x"}}"#
    ));
    assert_eq!(
        parse(&bad_source),
        Err(LockError::Invalid(LockField::LayerSource))
    );
}

#[test]
fn unknown_keys_are_ignored_and_not_written_back() {
    let text = format!(
        r#"{{"extra":1,"files":{{}},"layers":[{{"contentHash":"{HASH}","name":"x","other":[],"source":"path:./x"}}],"lockVersion":1,"visibility":"public"}}"#
    );
    let lock = parse(&text).expect("a valid lock");
    assert!(!lock.emit().contains("extra"));
    assert!(!lock.emit().contains("other"));
}

fn any_layer_name() -> impl Strategy<Value = LayerName> {
    "[a-z0-9]{1,4}([-.][a-z0-9]{1,3}){0,2}".prop_filter_map("a layer name", |text| {
        LayerName::try_from(text.as_str()).ok()
    })
}

fn any_source() -> impl Strategy<Value = Source> {
    prop_oneof![
        "path:[./a-z]{1,8}",
        "github:[a-z]{1,5}/[a-z]{1,5}@[a-z0-9.]{1,5}",
        "git:https://[a-z]{1,5}\\.test/[a-z]{1,5}@[a-z0-9.]{1,5}",
    ]
    .prop_filter_map("a source", |text| Source::try_from(text.as_str()).ok())
}

fn any_object_id() -> impl Strategy<Value = ObjectId> {
    prop_oneof![
        any::<[u8; 20]>().prop_map(ObjectId::Sha1),
        any::<[u8; 32]>().prop_map(ObjectId::Sha256),
    ]
}

fn any_hash_text() -> impl Strategy<Value = String> {
    any::<[u8; 32]>().prop_map(|bytes| format!("sha256:{}", crate::lock::hex::encode(&bytes)))
}

fn any_locked_layer() -> impl Strategy<Value = LockedLayer> {
    (
        any_layer_name(),
        any_source(),
        proptest::option::of(
            "[a-z0-9.]{1,6}".prop_filter_map("a ref", |text| GitRef::try_from(text.as_str()).ok()),
        ),
        proptest::option::of(any_object_id()),
        any_hash_text(),
    )
        .prop_filter_map("a locked layer", |(name, source, rev, sha, hash)| {
            Some(LockedLayer {
                name,
                source,
                rev,
                sha,
                content_hash: ContentHash::try_from(hash.as_str()).ok()?,
            })
        })
}

fn any_files() -> impl Strategy<Value = BTreeMap<SafeRelPath, FileHash>> {
    btree_map(
        vec("[a-zA-Z0-9 _.\u{e9}\u{4e2d}\u{1f600}\u{ff5e}-]{1,5}", 1..4)
            .prop_filter_map("a safe path", |segments| {
                SafeRelPath::try_from(segments.join("/").as_str()).ok()
            }),
        any_hash_text()
            .prop_filter_map("a file hash", |text| FileHash::try_from(text.as_str()).ok()),
        0..6,
    )
}

fn any_lock() -> impl Strategy<Value = Lock> {
    (
        prop_oneof![
            Just(Visibility::Public),
            Just(Visibility::Private),
            Just(Visibility::Confidential)
        ],
        vec(any_locked_layer(), 0..4),
        any_files(),
    )
        .prop_map(|(visibility, layers, files)| Lock {
            visibility,
            layers,
            files,
        })
}

proptest! {
    #[test]
    fn a_lock_that_is_emitted_reads_back_as_the_same_lock(lock in any_lock()) {
        let text = lock.emit();
        prop_assert_eq!(Lock::parse(text.as_bytes()), Ok(lock.clone()));
        // Emitting is a function of the lock: the second text is the first.
        prop_assert_eq!(Lock::parse(text.as_bytes()).map(|again| again.emit()), Ok(text));
    }
}
