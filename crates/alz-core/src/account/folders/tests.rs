use super::{AccountFolders, Relation};
use crate::path::AbsolutePath;

fn path(text: &str) -> AbsolutePath {
    AbsolutePath::try_from(text).expect("absolute")
}

fn folders() -> AccountFolders {
    AccountFolders::from_env(
        &path("/Users/u"),
        Some(&path("/work/claude-cfg")),
        Some(&path("/Users/u/dev/codex-cfg")),
    )
}

fn relation(text: &str) -> Option<Relation> {
    folders().relation(&path(text))
}

#[test]
fn the_account_folders_and_what_is_inside_them_touch() {
    for inside in [
        "/Users/u/.claude",
        "/Users/u/.claude/projects/x",
        "/Users/u/.claude.json",
        "/Users/u/.codex",
        "/Users/u/.codex/auth.json",
        "/Users/u/.claude-work",
        "/Users/u/.claude-work/skills",
        "/Users/u/.codex-client-a/x/y",
        "/Users/u/.claude-",
    ] {
        assert_eq!(relation(inside), Some(Relation::AccountFolder), "{inside}");
    }
}

#[test]
fn a_case_or_unicode_form_variant_of_an_account_folder_touches() {
    for variant in [
        "/Users/u/.CLAUDE-Work",
        "/Users/u/.Claude",
        "/Users/u/.CLAUDE.JSON",
        "/Users/u/.Claude.JSON/x",
    ] {
        assert_eq!(
            relation(variant),
            Some(Relation::AccountFolder),
            "{variant}"
        );
    }
    // The home in NFC and the path in NFD, and the reverse: APFS keeps either spelling.
    let nfc = AccountFolders::from_env(&path("/Users/\u{e9}"), None, None);
    let nfd = AccountFolders::from_env(&path("/Users/e\u{301}"), None, None);
    for (home, target) in [
        (&nfc, "/Users/e\u{301}/.claude-x"),
        (&nfd, "/Users/\u{e9}/.claude-x"),
    ] {
        assert_eq!(
            home.relation(&path(target)),
            Some(Relation::AccountFolder),
            "{target:?}"
        );
    }
    assert_eq!(nfc.relation(&path("/Users/E\u{301}")), Some(Relation::Home));
}

#[test]
fn the_folders_that_the_variables_name_touch_by_their_own_path() {
    assert_eq!(relation("/work/claude-cfg"), Some(Relation::Configured));
    assert_eq!(relation("/work/Claude-CFG/a/b"), Some(Relation::Configured));
    assert_eq!(
        relation("/Users/u/dev/codex-cfg/sessions"),
        Some(Relation::Configured)
    );
}

#[test]
fn home_and_everything_above_it_touch() {
    for above in ["/", "/Users", "/Users/u", "/users/U/", "/Users/u/./x/.."] {
        assert_eq!(relation(above), Some(Relation::Home), "{above}");
    }
}

#[test]
fn a_folder_that_holds_a_configured_folder_touches() {
    assert_eq!(relation("/work"), Some(Relation::Holds));
    assert_eq!(relation("/Users/u/dev"), Some(Relation::Holds));
}

#[test]
fn other_paths_do_not_touch() {
    for free in [
        "/srv/layers",
        "/Users/u/dev/repo",
        "/Users/u/.config/x",
        "/Users/u/claude",
        "/Users/u/.claudex",
        "/Users/u/.codex.json",
        "/Users/other/.claude",
        "/Users/u/dev/x/.claude-y2/..",
        "/work/other",
    ] {
        let relation = relation(free);
        // `/Users/u/dev/x/.claude-y2/..` is `/Users/u/dev/x`, which is free.
        assert_eq!(relation, None, "{free}");
    }
}

#[test]
fn dot_dot_is_applied_before_the_comparison() {
    assert_eq!(
        relation("/srv/../Users/u/.claude-w"),
        Some(Relation::AccountFolder)
    );
    assert_eq!(
        relation("/Users/u/dev/repo/../../.codex"),
        Some(Relation::AccountFolder)
    );
}

#[test]
fn the_real_spelling_of_home_touches_too() {
    let named = AccountFolders::from_env(&path("/var/h"), None, None)
        .with_real_home(&path("/private/var/h"))
        .with_real_configured(&path("/private/cfg"));
    assert_eq!(
        named.relation(&path("/private/var/h/.claude-x")),
        Some(Relation::AccountFolder)
    );
    assert_eq!(named.relation(&path("/private/var")), Some(Relation::Home));
    assert_eq!(
        named.relation(&path("/private/cfg/a")),
        Some(Relation::Configured)
    );
    assert!(!named.touches(&path("/private/var/other")));
}

#[test]
fn a_root_home_makes_every_name_below_it_an_account_candidate_only_by_name() {
    let root = AccountFolders::from_env(&path("/"), None, None);
    assert_eq!(root.relation(&path("/")), Some(Relation::Home));
    assert_eq!(
        root.relation(&path("/.claude-a")),
        Some(Relation::AccountFolder)
    );
    assert_eq!(root.relation(&path("/srv")), None);
}

#[test]
fn only_a_path_inside_an_account_folder_is_inside() {
    assert!(Relation::AccountFolder.is_inside());
    assert!(Relation::Configured.is_inside());
    assert!(!Relation::Home.is_inside());
    assert!(!Relation::Holds.is_inside());
}
