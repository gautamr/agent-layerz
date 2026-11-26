use super::{
    AllowCommand, Refusal, allowed_lines, already_allowed, not_a_person, nothing_to_remove,
    offer_lines, prompt, remove_offer_lines, removed,
};
use crate::account::AccountFamily;
use crate::account::resolution::{ExistingFolders, HomeFact, NoRepository};
use crate::account::{
    AccountName, AllowDecision, AllowEntry, AllowedSource, CachedDeclaration, CachedLayer,
    NonEmptyTargetMap, allowance, decide, decide_person_at_terminal,
};
use crate::env::EnvVars;
use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::merge::Placement;
use crate::path::AbsolutePath;
use crate::stack::{GitRef, ResolvedLayer, Source};

fn path(text: &str) -> AbsolutePath {
    AbsolutePath::try_from(text).expect("an absolute path")
}

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("a name")
}

fn home() -> HomeFact {
    HomeFact::Absolute {
        given: path("/home/p"),
        real: None,
    }
}

fn entry(root: &str, claude: &str, codex: &str, source: &str) -> AllowEntry {
    AllowEntry::new(
        path(root),
        NonEmptyTargetMap::from_entries([
            (AccountFamily::Claude, name(claude)),
            (AccountFamily::Codex, name(codex)),
        ])
        .expect("a map"),
        AllowedSource::try_from(source).expect("a source"),
    )
}

fn declared(account: &str, recorded: &str) -> CachedDeclaration {
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
        revision: None,
        recorded_source: recorded.to_owned(),
    }])
}

fn lines(messages: &[crate::Message]) -> Vec<String> {
    messages.iter().map(ToString::to_string).collect()
}

#[test]
fn the_offer_shows_the_accounts_the_layer_the_source_and_what_is_recorded() {
    let AllowDecision::Ask(offer) = allowance(
        &path("/home/p/work/a"),
        &declared("{claude: client-a}", "/s"),
        Some(&entry("/home/p/work/a", "x", "x", "/old")),
    )
    .decision
    else {
        panic!("expected an offer");
    };
    let folders = ExistingFolders::new([name("client-a")], []);
    assert_eq!(
        lines(&offer_lines(&offer, &home(), &folders)),
        vec![
            "repository: ~/work/a",
            "accounts the layers name:",
            "  claude: client-a (the account folder exists)",
            "  codex: not named by the layers (the other sources decide)",
            "declared by the layer \"first\", stack entry 1",
            "  source as written: \"path:../first\"",
            "  ref: \"v1\"",
            "  revision: none (unpinned path: layer)",
            "  recorded source: \"/s\"",
            "currently allowed: claude=x, codex=x from \"/old\"",
            "Claude Code and Codex started here will use these accounts' logins and MCP connections.",
        ]
    );
}

#[test]
fn a_missing_folder_and_the_default_account_are_told_apart() {
    let AllowDecision::Ask(offer) = allowance(
        &path("/r"),
        &declared("{claude: default, codex: other}", "/s"),
        None,
    )
    .decision
    else {
        panic!("expected an offer");
    };
    let shown = lines(&offer_lines(&offer, &home(), &ExistingFolders::default()));
    assert!(shown.contains(&"  claude: default (the default account)".to_owned()));
    assert!(shown.contains(&"  codex: other (the account folder does not exist yet)".to_owned()));
}

#[test]
fn the_prompt_repeats_the_unshortened_root_and_the_source_quoted() {
    let recorded = entry("/home/p/work/a", "client-a", "client-a", "/s");
    assert_eq!(
        prompt(AllowCommand::Allow, &name("client-a"), &recorded).to_string(),
        "Type client-a to allow it for \"/home/p/work/a\" (source \"/s\"): "
    );
    assert_eq!(
        prompt(AllowCommand::Remove, &name("client-a"), &recorded).to_string(),
        "Type client-a to remove it for \"/home/p/work/a\" (source \"/s\"): "
    );
}

#[test]
fn text_from_the_repository_is_quoted_with_every_other_character_escaped() {
    let odd = entry("/r/\u{85}x", "a", "a", "/s/\u{2028}");
    let text = prompt(AllowCommand::Allow, &name("a"), &odd).to_string();
    assert_eq!(
        text,
        "Type a to allow it for \"/r/\\u{0085}x\" (source \"/s/\\u{2028}\"): "
    );
}

#[test]
fn after_a_write_a_note_names_each_account_folder_that_does_not_exist() {
    let recorded = entry("/r", "client-a", "default", "/s");
    assert_eq!(
        lines(&allowed_lines(
            &recorded,
            &home(),
            &ExistingFolders::default()
        )),
        vec![
            "allowed: /r uses claude=client-a, codex=default from \"/s\"",
            "note: the claude account folder for client-a does not exist yet; create it with: alz account init client-a",
        ]
    );
    assert_eq!(
        allowed_lines(
            &recorded,
            &home(),
            &ExistingFolders::new([name("client-a")], [])
        )
        .len(),
        1
    );
}

#[test]
fn the_short_lines_name_the_root_with_a_tilde() {
    let root = path("/home/p/r");
    assert_eq!(
        already_allowed(&root, &home()).to_string(),
        "already allowed: ~/r"
    );
    assert_eq!(
        nothing_to_remove(&root, &home()).to_string(),
        "nothing to remove: ~/r has no entry in allowed.yaml"
    );
    assert_eq!(
        removed(&root, &home()).to_string(),
        "removed the entry for ~/r from allowed.yaml"
    );
}

#[test]
fn remove_shows_the_recorded_entry() {
    assert_eq!(
        lines(&remove_offer_lines(&entry("/r", "a", "b", "/s"), &home())),
        vec![
            "repository: /r",
            "allowed now: claude=a, codex=b from \"/s\"",
            "removing it sends this repository back to the default account unless accounts.yaml names one",
        ]
    );
}

#[test]
fn a_refusal_names_every_reason_and_the_markers_by_name_only() {
    let env = EnvVars::default()
        .with("CODEX_CI", "secret-value")
        .with("AI_AGENT", "");
    let reason = decide_person_at_terminal(false, &env).expect_err("not a person");
    let refusal = not_a_person(AllowCommand::Remove, &reason);
    assert_eq!(
        refusal.headline.to_string(),
        "alz account allow --remove must be run by a person at a terminal: standard input is not a terminal; an agent session is running here (AI_AGENT, CODEX_CI set)"
    );
    assert_eq!(lines(&refusal.following), vec!["nothing was changed"]);
}

#[test]
fn claude_code_adds_the_note_about_integrated_terminals() {
    let env = EnvVars::default().with("CLAUDECODE", "1");
    let reason = decide_person_at_terminal(true, &env).expect_err("an agent session");
    let refusal = not_a_person(AllowCommand::Allow, &reason);
    assert_eq!(
        refusal.headline.to_string(),
        "alz account allow must be run by a person at a terminal: an agent session is running here (CLAUDECODE set)"
    );
    assert_eq!(
        lines(&refusal.following),
        vec![
            "Claude Code also sets CLAUDECODE in an IDE's integrated terminal; run alz account allow in a terminal window outside the IDE",
            "nothing was changed",
        ]
    );
}

#[test]
fn each_refusal_has_the_words_of_the_spec() {
    let root = path("/home/p/r");
    let cases = [
        (
            Refusal::HomeNotAbsolute,
            "HOME is not an absolute path; nothing was changed",
        ),
        (
            Refusal::NoRepository(NoRepository::NotFound),
            "repository: none (no repository here); run it inside a repository",
        ),
        (
            Refusal::Repair(&root),
            "repair ~/r/layer.yaml first; it cannot be used",
        ),
        (
            Refusal::NothingDeclared { has_entry: false },
            "the layers name no account here; nothing was changed",
        ),
        (
            Refusal::NothingDeclared { has_entry: true },
            "the layers name no account here; nothing was changed; to withdraw the earlier approval run: alz account allow --remove",
        ),
        (
            Refusal::Changed,
            "the declaration changed while you were confirming; run alz account allow again",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.message(&home()).to_string(), text);
    }
}
