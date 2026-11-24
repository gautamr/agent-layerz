use super::{AllowDecision, allowance, distinct_names};
use crate::account::{
    AccountName, AllowEntry, AllowedSource, CachedDeclaration, CachedLayer, Miss, MissReason,
    NonEmptyTargetMap, decide,
};
use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::lock::ObjectId;
use crate::merge::Placement;
use crate::path::AbsolutePath;
use crate::stack::{GitRef, ResolvedLayer, Source};
use crate::target::TargetName;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn root() -> AbsolutePath {
    AbsolutePath::try_from("/work/a").expect("an absolute path")
}

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("a name")
}

fn declared(account: &str, recorded: &str, revision: Option<&str>) -> CachedDeclaration {
    let yaml = format!("name: first\nkind: personal\nvisibility: private\naccount: {account}\n");
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    let meta = LayerMeta::from_yaml(&node).expect("a layer");
    decide(&[CachedLayer {
        position: 0,
        placement: Placement::Outside,
        layer: ResolvedLayer::new(
            Source::try_from("path:../first").expect("a source"),
            meta,
            LayerContents::default(),
            None,
        ),
        git_ref: GitRef::try_from("v1").ok(),
        revision: revision.map(|text| ObjectId::try_from(text).expect("an id")),
        recorded_source: recorded.to_owned(),
    }])
}

fn entry(claude: &str, codex: &str, source: &str) -> AllowEntry {
    AllowEntry::new(
        root(),
        NonEmptyTargetMap::from_entries([
            (TargetName::Claude, name(claude)),
            (TargetName::Codex, name(codex)),
        ])
        .expect("a map"),
        AllowedSource::try_from(source).expect("a source"),
    )
}

fn offer_of(decision: AllowDecision) -> Box<super::Offer> {
    let AllowDecision::Ask(offer) = decision else {
        panic!("expected an offer");
    };
    offer
}

#[test]
fn one_name_for_every_target_is_recorded_for_both_and_asked_once() {
    let found = allowance(&root(), &declared("client-a", "/s", Some(SHA)), None);
    let offer = offer_of(found.decision);
    assert_eq!(offer.entry(), &entry("client-a", "client-a", "/s"));
    assert_eq!(offer.names(), vec![name("client-a")]);
    assert_eq!(
        offer.revision().map(ToString::to_string).as_deref(),
        Some(SHA)
    );
    assert_eq!(offer.position(), 0);
}

#[test]
fn a_map_is_asked_once_for_each_distinct_name_in_target_order() {
    let found = allowance(
        &root(),
        &declared("{claude: client-a, codex: client-a-api}", "/s", None),
        None,
    );
    assert_eq!(
        offer_of(found.decision).names(),
        vec![name("client-a"), name("client-a-api")]
    );
}

#[test]
fn a_target_the_layer_leaves_out_is_not_recorded() {
    let found = allowance(&root(), &declared("{claude: client-a}", "/s", None), None);
    let offer = offer_of(found.decision);
    assert_eq!(offer.entry().account().get(TargetName::Codex), None);
}

#[test]
fn the_default_account_is_recorded_as_default() {
    let found = allowance(&root(), &declared("default", "/s", None), None);
    let offer = offer_of(found.decision);
    assert_eq!(
        offer.entry().account().get(TargetName::Claude),
        Some(&AccountName::default_account())
    );
}

#[test]
fn an_entry_equal_to_the_declaration_is_already_allowed() {
    let current = entry("client-a", "client-a", "/s");
    let found = allowance(&root(), &declared("client-a", "/s", None), Some(&current));
    assert_eq!(found.decision, AllowDecision::AlreadyAllowed);
}

#[test]
fn another_source_or_map_is_offered_with_the_entry_that_is_allowed_now() {
    let current = entry("client-a", "client-a", "/old");
    let found = allowance(&root(), &declared("client-a", "/s", None), Some(&current));
    assert_eq!(offer_of(found.decision).current(), Some(&current));
}

#[test]
fn the_reserved_name_is_refused() {
    let found = allowance(&root(), &declared("alz-blocked", "/s", None), None);
    assert_eq!(found.decision, AllowDecision::Reserved);
}

#[test]
fn no_declaration_says_so_and_knows_whether_there_is_an_entry_to_withdraw() {
    let none = CachedDeclaration::None {
        ignored: Vec::new(),
    };
    assert_eq!(
        allowance(&root(), &none, None).decision,
        AllowDecision::NothingDeclared { has_entry: false }
    );
    assert_eq!(
        allowance(&root(), &none, Some(&entry("a", "a", "/s"))).decision,
        AllowDecision::NothingDeclared { has_entry: true }
    );
}

#[test]
fn a_miss_is_a_miss_and_never_an_offer() {
    let miss = Miss::of(MissReason::LockMissing);
    assert_eq!(
        allowance(&root(), &CachedDeclaration::Miss(miss), None).decision,
        AllowDecision::CacheMiss(miss)
    );
}

#[test]
fn the_declaration_read_again_must_be_what_was_shown() {
    let shown = offer_of(allowance(&root(), &declared("client-a", "/s", Some(SHA)), None).decision);
    assert!(shown.is_still_declared_by(&root(), &declared("client-a", "/s", Some(SHA))));
    for changed in [
        declared("client-b", "/s", Some(SHA)),
        declared("client-a", "/t", Some(SHA)),
        declared("client-a", "/s", None),
        declared("{claude: client-a}", "/s", Some(SHA)),
        CachedDeclaration::Miss(Miss::of(MissReason::LockMissing)),
        CachedDeclaration::None {
            ignored: Vec::new(),
        },
    ] {
        assert!(!shown.is_still_declared_by(&root(), &changed));
    }
}

#[test]
fn the_entry_the_file_holds_now_does_not_change_what_was_shown() {
    let shown = offer_of(allowance(&root(), &declared("client-a", "/s", None), None).decision);
    // Another run may have recorded it meanwhile: the writer, not this check, decides that.
    assert!(shown.is_still_declared_by(&root(), &declared("client-a", "/s", None)));
}

#[test]
fn distinct_names_keep_target_order() {
    let map = entry("b", "a", "/s");
    assert_eq!(distinct_names(map.account()), vec![name("b"), name("a")]);
    assert_eq!(
        distinct_names(entry("a", "a", "/s").account()),
        vec![name("a")]
    );
}
