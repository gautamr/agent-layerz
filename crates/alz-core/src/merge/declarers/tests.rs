//! Who may declare `commit_identity` and `account` (§7.12, §7.13): the merge cases of acceptance
//! tests 26 ("who may declare it", "generated Commits rule") and 29 ("who wins"), over in-memory
//! stacks whose layers lie outside the repository except the ones a test names, plus the laws of
//! the rule.
//!
//! The cases that need a real source, a link or git (a symlink into the repository, a `git:` path,
//! a case variant) are placements, which `alz-io` decides and tests; the cases that print a
//! generated section or set a git config are R5's and R6's.

use proptest::collection::vec;
use proptest::prelude::{Just, Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest};

use super::{Candidate, Declarations, Declared, Reason};
use crate::account::AccountDeclaration;
use crate::identity::CommitIdentity;
use crate::layer::{Kind, LayerName, Visibility};
use crate::merge::Merging;
use crate::merge::placement::Placement;
use crate::merge::testing::{layer_from_yaml, lines, merged_placed, origin_of};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;

const ERROR_HINT: &str =
    ". Only the first layer, or one private layer with kind: client outside the repository, may";

/// The `commit_identity` of the fixtures of test 26: the client's, and the owner's.
const CLIENT_IDENTITY: &str =
    "commit_identity:\n  name: Owner Example\n  email: owner@client.example\n";
const OWNER_IDENTITY: &str =
    "commit_identity:\n  name: Owner Example\n  email: owner@example.com\n";

/// A layer with the metadata `name`, `kind` and `visibility` and `extra` lines after them.
fn declaring(name: &str, kind: &str, visibility: &str, extra: &str) -> ResolvedLayer {
    layer_from_yaml(
        &format!("name: {name}\nkind: {kind}\nvisibility: {visibility}\n{extra}"),
        &[],
    )
}

/// The first layer of test 26's fixture: public, kind `personal`.
fn personal(extra: &str) -> ResolvedLayer {
    declaring("personal", "personal", "public", extra)
}

fn client(name: &str, extra: &str) -> ResolvedLayer {
    declaring(name, "client", "private", extra)
}

/// The repository's own layer, which declares nothing unless a test says so.
fn repo(kind: &str, extra: &str) -> ResolvedLayer {
    declaring("repo", kind, "private", extra)
}

fn identity(found: &Merging) -> &Declarations<CommitIdentity> {
    found.merged().expect("merged").commit_identity()
}

fn account(found: &Merging) -> &Declarations<AccountDeclaration> {
    found.merged().expect("merged").account()
}

fn winner_name<T>(declarations: &Declarations<T>) -> Option<&str> {
    declarations
        .declared()
        .winner()
        .map(|(layer, _)| layer.as_str())
}

fn email_of(declarations: &Declarations<CommitIdentity>) -> String {
    declarations
        .declared()
        .winner()
        .map(|(_, identity)| identity.email().as_str().to_owned())
        .expect("a winner")
}

// ---- test 26, "who may declare it (§7.12)" ----

#[test]
fn a_declaration_in_the_repo_layer_is_refused_whatever_its_kind_says() {
    // Kind `client`: inside the repository is the only reason.
    let found = merged_placed(
        vec![personal(OWNER_IDENTITY), repo("client", CLIENT_IDENTITY)],
        &["repo"],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] repo:layer.yaml: layer 'repo' cannot declare commit_identity: it is inside the repository{ERROR_HINT}"
        )]
    );
    assert!(found.merged().is_none());
    // Any other kind adds its own reason.
    let found = merged_placed(
        vec![personal(OWNER_IDENTITY), repo("repo", CLIENT_IDENTITY)],
        &["repo"],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] repo:layer.yaml: layer 'repo' cannot declare commit_identity: it is inside the repository; its kind is 'repo', not 'client'{ERROR_HINT}"
        )]
    );
}

#[test]
fn a_declaration_in_a_group_layer_is_refused_and_names_its_kind() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", CLIENT_IDENTITY),
            declaring("group", "group", "private", CLIENT_IDENTITY),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] group:layer.yaml: layer 'group' cannot declare commit_identity: its kind is 'group', not 'client'{ERROR_HINT}"
        )]
    );
}

#[test]
fn a_public_layer_that_calls_itself_client_is_refused() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            declaring("client", "client", "public", CLIENT_IDENTITY),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] client:layer.yaml: layer 'client' cannot declare commit_identity: it is public{ERROR_HINT}"
        )]
    );
}

#[test]
fn a_second_client_layer_is_refused_and_both_are_named() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", CLIENT_IDENTITY),
            client("client2", CLIENT_IDENTITY),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [
            "error [identity] client2:layer.yaml: layers 'client' and 'client2' both declare commit_identity; only one client layer may"
        ]
    );
}

#[test]
fn every_client_layer_is_named_when_more_than_two_declare() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", CLIENT_IDENTITY),
            client("client2", CLIENT_IDENTITY),
            client("client3", CLIENT_IDENTITY),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [
            "error [identity] client2:layer.yaml: layers 'client', 'client2' and 'client3' all declare commit_identity; only one client layer may"
        ]
    );
}

#[test]
fn a_client_layer_inside_the_repository_is_refused() {
    // The placement of a link into the repository, a case variant and a local `git:` path is
    // decided in alz-io; here the layer is inside.
    let found = merged_placed(
        vec![personal(OWNER_IDENTITY), client("client", CLIENT_IDENTITY)],
        &["client"],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] client:layer.yaml: layer 'client' cannot declare commit_identity: it is inside the repository{ERROR_HINT}"
        )]
    );
}

#[test]
fn a_first_layer_inside_the_repository_is_refused_and_leaves_no_winner() {
    let found = merged_placed(
        vec![personal(OWNER_IDENTITY), client("client", CLIENT_IDENTITY)],
        &["personal"],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [identity] personal:layer.yaml: layer 'personal' cannot declare commit_identity: it is inside the repository{ERROR_HINT}"
        )]
    );
    assert!(found.merged().is_none());
}

#[test]
fn a_kind_with_control_characters_prints_without_them() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            declaring(
                "group",
                r#""gro\u200bup\u202e""#,
                "private",
                CLIENT_IDENTITY,
            ),
        ],
        &[],
    );
    let text = lines(&found).join("\n");
    assert!(text.contains("group:layer.yaml"));
    assert!(!text.contains('\u{200b}'));
    assert!(!text.contains('\u{202e}'));
}

// ---- test 26, "generated Commits rule (§7.12)": which declaration wins ----

#[test]
fn the_client_layer_overrides_the_first_layer_and_is_the_winner() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", CLIENT_IDENTITY),
            repo("repo", ""),
        ],
        &["repo"],
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(winner_name(identity(&found)), Some("client"));
    assert_eq!(email_of(identity(&found)), "owner@client.example");
}

#[test]
fn the_first_layers_identity_applies_when_no_client_layer_declares() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", ""),
            repo("repo", ""),
        ],
        &["repo"],
    );
    assert_eq!(winner_name(identity(&found)), Some("personal"));
    assert_eq!(email_of(identity(&found)), "owner@example.com");
}

#[test]
fn no_declaration_is_no_winner_and_no_error() {
    let found = merged_placed(
        vec![personal(""), client("client", ""), repo("repo", "")],
        &["repo"],
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(identity(&found).declared(), &Declared::None);
    assert_eq!(account(&found).declared(), &Declared::None);
}

// ---- test 29, "who wins (§7.13 'Who may declare it')" ----

#[test]
fn the_first_layers_account_applies_when_no_client_layer_declares() {
    let found = merged_placed(
        vec![
            personal("account: personal-a\n"),
            client("client", ""),
            repo("repo", ""),
        ],
        &["repo"],
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(winner_name(account(&found)), Some("personal"));
}

#[test]
fn the_client_layers_whole_account_replaces_the_first_layers_map_included() {
    let found = merged_placed(
        vec![
            personal("account:\n  claude: personal-a\n  codex: personal-b\n"),
            client("client", "account:\n  claude: client-a\n"),
        ],
        &[],
    );
    let (layer, value) = account(&found).declared().winner().expect("a winner");
    assert_eq!(layer.as_str(), "client");
    // The maps are never mixed: the first layer's codex name does not survive.
    assert!(value.choice_for(TargetName::Claude).is_some());
    assert_eq!(value.choice_for(TargetName::Codex), None);
}

#[test]
fn a_confidential_client_layer_may_declare() {
    let found = merged_placed(
        vec![
            personal("account: personal-a\n"),
            declaring("client", "client", "confidential", "account: client-a\n"),
        ],
        &[],
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    assert_eq!(winner_name(account(&found)), Some("client"));
}

#[test]
fn a_public_client_layer_may_not_declare_an_account() {
    let found = merged_placed(
        vec![
            personal("account: personal-a\n"),
            declaring("public-client", "client", "public", "account: client-a\n"),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [account] public-client:layer.yaml: layer 'public-client' cannot declare account: it is public{ERROR_HINT}"
        )]
    );
}

#[test]
fn a_group_layer_may_not_declare_an_account() {
    let found = merged_placed(
        vec![
            personal("commit_identity:\n  name: Owner Example\n  email: owner@example.com\n"),
            declaring("group", "group", "private", "account: group-a\n"),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [format!(
            "error [account] group:layer.yaml: layer 'group' cannot declare account: its kind is 'group', not 'client'{ERROR_HINT}"
        )]
    );
    // A refused declaration is a build error, so there is no merged model to read a winner from.
    assert!(found.merged().is_none());
}

#[test]
fn two_client_layers_conflict_over_the_account_and_name_both() {
    let found = merged_placed(
        vec![
            personal("account: personal-a\n"),
            client("client", "account: client-a\n"),
            client("client-b", "account: client-b\n"),
        ],
        &[],
    );
    assert_eq!(
        lines(&found),
        [
            "error [account] client-b:layer.yaml: layers 'client' and 'client-b' both declare account; only one client layer may"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn the_two_fields_follow_the_rule_independently() {
    let found = merged_placed(
        vec![
            personal(OWNER_IDENTITY),
            client("client", "account: client-a\n"),
        ],
        &[],
    );
    assert_eq!(winner_name(identity(&found)), Some("personal"));
    assert_eq!(winner_name(account(&found)), Some("client"));
}

#[test]
fn a_layer_with_no_placement_cannot_declare() {
    // `Placements::default()` places nobody: the stricter answer, so it never lets a layer in.
    let found = crate::merge::testing::merged(vec![personal(OWNER_IDENTITY)]);
    assert!(found.merged().is_none());
    assert_eq!(lines(&found).len(), 1);
}

// ---- the laws ----

fn kind() -> impl Strategy<Value = Kind> {
    prop_oneof![
        Just(Kind::Client),
        Just(Kind::try_from("group").expect("a kind")),
        Just(Kind::try_from("repo").expect("a kind")),
    ]
}

fn visibility() -> impl Strategy<Value = Visibility> {
    prop_oneof![
        Just(Visibility::Public),
        Just(Visibility::Private),
        Just(Visibility::Confidential),
    ]
}

fn placement() -> impl Strategy<Value = Placement> {
    prop_oneof![Just(Placement::Inside), Just(Placement::Outside)]
}

/// The layers that declare, in stack order: the first has index 0 when `first_is_there`.
fn candidates() -> impl Strategy<Value = Vec<Candidate<u8>>> {
    (
        any::<bool>(),
        vec((kind(), visibility(), placement(), any::<u8>()), 0..6),
    )
        .prop_map(|(first_is_there, rest)| {
            rest.into_iter()
                .enumerate()
                .map(|(index, (kind, visibility, placement, value))| {
                    let layer = LayerName::try_from(format!("layer-{index}").as_str())
                        .expect("a layer name");
                    Candidate {
                        origin: origin_of(layer.clone()),
                        layer,
                        is_first: first_is_there && index == 0,
                        kind,
                        visibility,
                        placement,
                        value,
                    }
                })
                .collect()
        })
}

fn may_declare(candidate: &Candidate<u8>) -> bool {
    candidate.placement == Placement::Outside
        && (candidate.is_first
            || (candidate.kind.is_client() && candidate.visibility >= Visibility::Private))
}

proptest! {
    #[test]
    fn a_winner_is_the_first_layer_or_a_private_client_layer_outside_the_repository(
        candidates in candidates(),
    ) {
        let declared: Vec<_> = candidates.clone();
        let found = Declarations::evaluate(candidates);
        if let Some((layer, value)) = found.declared().winner() {
            let winner = declared
                .iter()
                .find(|candidate| &candidate.layer == layer)
                .expect("a layer that declared");
            prop_assert!(may_declare(winner));
            prop_assert_eq!(&winner.value, value);
            prop_assert!(winner.is_first || winner.kind.is_client());
            prop_assert_eq!(winner.placement, Placement::Outside);
        }
    }

    #[test]
    fn there_is_a_conflict_exactly_when_several_clients_declare_or_the_first_layer_is_refused(
        candidates in candidates(),
    ) {
        let several = candidates
            .iter()
            .filter(|candidate| !candidate.is_first && may_declare(candidate))
            .count()
            > 1;
        let first_refused = candidates
            .iter()
            .any(|candidate| candidate.is_first && !may_declare(candidate));
        let found = Declarations::evaluate(candidates);
        prop_assert_eq!(found.declared().is_conflict(), several || first_refused);
    }

    #[test]
    fn a_client_wins_over_the_first_layer_and_the_refused_layers_never_win(
        candidates in candidates(),
    ) {
        let eligible_clients: Vec<_> = candidates
            .iter()
            .filter(|candidate| !candidate.is_first && may_declare(candidate))
            .map(|candidate| candidate.layer.clone())
            .collect();
        let first_refused = candidates
            .iter()
            .any(|candidate| candidate.is_first && !may_declare(candidate));
        let found = Declarations::evaluate(candidates);
        prop_assert_eq!(found.clients(), eligible_clients.as_slice());
        if let ([only], false) = (eligible_clients.as_slice(), first_refused) {
            prop_assert_eq!(found.declared().winner().map(|(layer, _)| layer), Some(only));
        }
        for refused in found.refused() {
            prop_assert!(!refused.reasons().is_empty());
            prop_assert!(found.declared().winner().is_none_or(|(layer, _)| layer != refused.layer()));
        }
    }
}

#[test]
fn a_refused_kind_is_kept_as_the_reason() {
    let group = LayerName::try_from("group").expect("a layer name");
    let found = Declarations::evaluate(vec![Candidate {
        origin: origin_of(group.clone()),
        layer: group,
        is_first: false,
        kind: Kind::try_from("group").expect("a kind"),
        visibility: Visibility::Public,
        placement: Placement::Inside,
        value: 1_u8,
    }]);
    assert_eq!(
        found.refused()[0].reasons(),
        [
            Reason::InsideRepository,
            Reason::KindNotClient(Kind::try_from("group").expect("a kind")),
            Reason::Public,
        ]
    );
    assert_eq!(found.declared(), &Declared::None);
}
