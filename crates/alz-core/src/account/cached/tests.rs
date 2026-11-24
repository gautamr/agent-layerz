use std::collections::BTreeSet;

use super::{
    CachedDeclaration, CachedLayer, IndexEntry, Miss, MissReason, ReadPlan, decide,
    has_parent_segment, lock_entry, lock_miss, read_plan,
};
use crate::layer::{LayerContents, LayerMeta, LayerRefusal};
use crate::lint::parse_document;
use crate::lock::{ContentHash, LayerFiles, Lock, LockError, LockField, LockedLayer, ObjectId};
use crate::merge::{Placement, Reason};
use crate::stack::{GitRef, ResolvedLayer, Source, StackEntry};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn id(text: &str) -> ObjectId {
    ObjectId::try_from(text).expect("an object id")
}

fn source(text: &str) -> Source {
    Source::try_from(text).expect("a source")
}

fn pin(text: &str) -> GitRef {
    GitRef::try_from(text).expect("a ref")
}

fn locked(text: &str, rev: Option<&str>, sha: Option<&str>) -> LockedLayer {
    LockedLayer {
        name: crate::layer::LayerName::try_from("x").expect("a name"),
        source: source(text),
        rev: rev.map(pin),
        sha: sha.map(id),
        content_hash: ContentHash::of(&LayerFiles::default()),
    }
}

fn lock_of(layers: Vec<LockedLayer>) -> Lock {
    Lock {
        visibility: crate::layer::Visibility::Public,
        layers,
        files: std::collections::BTreeMap::new(),
    }
}

#[test]
fn every_miss_has_its_own_code_and_phrase() {
    let all = [
        MissReason::AccountFolder,
        MissReason::LockMissing,
        MissReason::LockUnreadable,
        MissReason::SourceInvalid,
        MissReason::LockEntryMissing,
        MissReason::LockEntryDuplicate,
        MissReason::LockShaInvalid,
        MissReason::LockRefMismatch,
        MissReason::CacheInRepo,
        MissReason::LayerDirectoryMissing,
        MissReason::IndexMissing,
        MissReason::IndexInvalid,
        MissReason::CacheEntryMissing,
        MissReason::CacheEntryNotDirectory,
        MissReason::ContentHash,
        MissReason::LayerInvalid,
        MissReason::StackInvalid,
        MissReason::Unreadable,
        MissReason::CacheLocation,
    ];
    let codes: BTreeSet<_> = all.iter().map(|miss| miss.code()).collect();
    let phrases: BTreeSet<_> = all.iter().map(|miss| miss.phrase()).collect();
    assert_eq!((codes.len(), phrases.len()), (all.len(), all.len()));
    assert_eq!(
        MissReason::ContentHash.phrase(),
        "a layer no longer matches layer.lock"
    );
    assert_eq!(MissReason::LayerInvalid.code(), "layer-invalid");
}

#[test]
fn a_refusal_of_the_layer_is_the_miss_the_order_of_checks_names() {
    let misses = [
        (LayerRefusal::TooManyEntries, MissReason::Unreadable),
        (LayerRefusal::NameNotUtf8, MissReason::LayerInvalid),
        (LayerRefusal::NameControlOrFormat, MissReason::LayerInvalid),
        (LayerRefusal::NameNotSafe, MissReason::LayerInvalid),
        (LayerRefusal::Unreadable, MissReason::Unreadable),
        (LayerRefusal::EnvFile, MissReason::LayerInvalid),
    ];
    for (refusal, miss) in misses {
        assert_eq!(MissReason::from(refusal), miss);
        assert_eq!(miss.code(), refusal.miss());
    }
}

#[test]
fn a_lock_that_does_not_parse_is_unreadable_whatever_field_is_bad() {
    // A short, upper case or empty `sha` makes the whole lock invalid (§5.3), so it is "cannot be
    // read"; "holds a commit id that is not valid" is for an entry that parses but has no usable
    // `sha` for its layer (test 30).
    assert_eq!(
        lock_miss(&LockError::Invalid(LockField::LayerSha)),
        MissReason::LockUnreadable
    );
    assert_eq!(
        lock_miss(&LockError::Invalid(LockField::LayerContentHash)),
        MissReason::LockUnreadable
    );
}

#[test]
fn a_source_needs_exactly_one_lock_entry() {
    let lock = lock_of(vec![
        locked("path:a", None, None),
        locked("path:b", None, None),
        locked("path:b", None, None),
    ]);
    assert!(lock_entry(&lock, &source("path:a")).is_ok());
    assert_eq!(
        lock_entry(&lock, &source("path:c")).unwrap_err(),
        MissReason::LockEntryMissing
    );
    assert_eq!(
        lock_entry(&lock, &source("path:b")).unwrap_err(),
        MissReason::LockEntryDuplicate
    );
}

#[test]
fn a_layer_is_read_from_the_entry_the_lock_pins() {
    let git = StackEntry::new(source("github:o/r@v1"), None);
    assert_eq!(
        read_plan(&git, &locked("github:o/r@v1", Some("v1"), Some(SHA))),
        Ok(ReadPlan::Git(id(SHA)))
    );
    assert_eq!(
        read_plan(&git, &locked("github:o/r@v1", Some("v1"), None)),
        Err(MissReason::LockShaInvalid)
    );
    let pinned = StackEntry::new(source("path:../l"), Some(pin("v1")));
    assert_eq!(
        read_plan(&pinned, &locked("path:../l", Some("v1"), Some(SHA256))),
        Ok(ReadPlan::Indexed(id(SHA256)))
    );
    assert_eq!(
        read_plan(&pinned, &locked("path:../l", Some("v2"), Some(SHA))),
        Err(MissReason::LockRefMismatch)
    );
    assert_eq!(
        read_plan(&pinned, &locked("path:../l", None, Some(SHA))),
        Err(MissReason::LockRefMismatch)
    );
    assert_eq!(
        read_plan(&pinned, &locked("path:../l", Some("v1"), None)),
        Err(MissReason::LockShaInvalid)
    );
    let plain = StackEntry::new(source("path:../l"), None);
    assert_eq!(
        read_plan(&plain, &locked("path:../l", None, None)),
        Ok(ReadPlan::InPlace)
    );
    assert_eq!(
        read_plan(&plain, &locked("path:../l", None, Some(SHA))),
        Err(MissReason::LockShaInvalid)
    );
}

#[test]
fn a_git_url_with_a_parent_segment_is_refused_whatever_its_scheme() {
    assert!(has_parent_segment(&source("git:https://h/a/../b.git@v1")));
    assert!(has_parent_segment(&source("git:../x@v1")));
    assert!(!has_parent_segment(&source("git:https://h/a..b/c.git@v1")));
    assert!(!has_parent_segment(&source("path:../x")));
}

#[test]
fn the_index_names_a_tree_as_long_as_the_commit() {
    let sha1 = id(SHA);
    let sha256 = id(SHA256);
    assert_eq!(IndexEntry::Tree(sha1).tree_for(sha1), Ok(sha1));
    assert_eq!(IndexEntry::Tree(sha256).tree_for(sha256), Ok(sha256));
    assert_eq!(
        IndexEntry::Tree(sha256).tree_for(sha1),
        Err(MissReason::IndexInvalid)
    );
    assert_eq!(
        IndexEntry::Tree(sha1).tree_for(sha256),
        Err(MissReason::IndexInvalid)
    );
    assert_eq!(
        IndexEntry::Invalid.tree_for(sha1),
        Err(MissReason::IndexInvalid)
    );
    assert_eq!(
        IndexEntry::Missing.tree_for(sha1),
        Err(MissReason::IndexMissing)
    );
    assert_eq!(
        IndexEntry::NotOwned.tree_for(sha1),
        Err(MissReason::Unreadable)
    );
}

fn layer(position: usize, placement: Placement, yaml: &str) -> CachedLayer {
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    let meta = LayerMeta::from_yaml(&node).expect("a layer");
    let name = meta.name().to_string();
    CachedLayer {
        position,
        placement,
        layer: ResolvedLayer::new(
            source(&format!("path:../{name}")),
            meta,
            LayerContents::default(),
            None,
        ),
        git_ref: None,
        revision: None,
        recorded_source: format!("/layers/{name}"),
    }
}

fn first(account: &str) -> String {
    format!("name: first\nkind: personal\nvisibility: private\n{account}")
}

fn client(name: &str, visibility: &str) -> String {
    format!("name: {name}\nkind: client\nvisibility: {visibility}\naccount: {name}\n")
}

fn declared(result: &CachedDeclaration) -> Option<String> {
    match result {
        CachedDeclaration::Declared { declaration, .. } => Some(declaration.layer.to_string()),
        CachedDeclaration::Miss(_)
        | CachedDeclaration::None { .. }
        | CachedDeclaration::Conflict { .. } => None,
    }
}

#[test]
fn the_first_layer_outside_the_repository_declares() {
    let result = decide(&[layer(0, Placement::Outside, &first("account: work\n"))]);
    assert_eq!(declared(&result).as_deref(), Some("first"));
}

#[test]
fn a_client_layer_replaces_the_first_layers_declaration() {
    let result = decide(&[
        layer(0, Placement::Outside, &first("account: work\n")),
        layer(1, Placement::Outside, &client("acme", "private")),
    ]);
    assert_eq!(declared(&result).as_deref(), Some("acme"));
}

#[test]
fn two_client_layers_are_a_conflict_and_never_no_declaration() {
    let result = decide(&[
        layer(0, Placement::Outside, &first("")),
        layer(1, Placement::Outside, &client("acme", "private")),
        layer(2, Placement::Outside, &client("beta", "confidential")),
    ]);
    let CachedDeclaration::Conflict { clients, .. } = result else {
        panic!("expected a conflict: {result:?}");
    };
    assert_eq!(clients.len(), 2);
}

#[test]
fn a_declaration_inside_the_repository_is_reported_and_neither_wins_nor_conflicts() {
    let result = decide(&[
        layer(0, Placement::Inside, &first("account: work\n")),
        layer(1, Placement::Outside, &client("acme", "private")),
        layer(2, Placement::Inside, &client("beta", "private")),
    ]);
    let CachedDeclaration::Declared {
        declaration,
        ignored,
    } = result
    else {
        panic!("expected a declaration: {result:?}");
    };
    assert_eq!(declaration.layer.as_str(), "acme");
    assert_eq!(
        ignored
            .iter()
            .map(|found| found.position)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    assert!(
        ignored
            .iter()
            .all(|found| found.reasons.contains(&Reason::InsideRepository))
    );
}

#[test]
fn a_public_or_non_client_later_layer_is_ignored_with_its_reasons() {
    let result = decide(&[
        layer(
            0,
            Placement::Outside,
            "name: first\nkind: personal\nvisibility: public\n",
        ),
        layer(1, Placement::Outside, &client("acme", "public")),
    ]);
    let CachedDeclaration::None { ignored } = result else {
        panic!("expected no declaration: {result:?}");
    };
    assert_eq!(ignored.len(), 1);
    assert_eq!(ignored[0].reasons, [Reason::Public]);
}

#[test]
fn layers_outside_the_repository_that_do_not_make_a_stack_are_a_miss() {
    let result = decide(&[
        layer(0, Placement::Outside, &first("")),
        layer(1, Placement::Outside, &first("")),
    ]);
    assert_eq!(
        result,
        CachedDeclaration::Miss(Miss::of(MissReason::StackInvalid))
    );
}

#[test]
fn layers_inside_the_repository_are_not_validated() {
    let result = decide(&[
        layer(0, Placement::Outside, &first("")),
        layer(1, Placement::Inside, &first("")),
    ]);
    assert!(matches!(result, CachedDeclaration::None { .. }));
}
