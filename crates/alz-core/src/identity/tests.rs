//! The laws of the identity step across its modules (spec §9.4): no report holds a value git
//! reported, a fix command appears exactly for a shell-safe declared value, findings print in one
//! order whatever order they came in, and the setup plan never names a key that is present.

use std::hint::black_box;

use proptest::prelude::{Just, Strategy, any, prop_assert, prop_assert_eq, proptest};

use super::{
    Command, ConfigAnswer, ConfigOrigin, DeclaredBy, DeclaredIdentity, Effective, EmailAddress,
    Field, Finding, Findings, Home, IdentityCheck, IdentityName, KeyState, Presence, RepoState,
    Report, SetupPlan, ShellSafe, Slot, Source, Verdicts, locate,
};
use crate::layer::LayerName;
use crate::stack::Source as StackSource;

fn declared(name: &str, email: &str) -> DeclaredIdentity {
    DeclaredIdentity::new(
        DeclaredBy::new(
            &LayerName::try_from("client").unwrap(),
            &StackSource::try_from("path:../client").unwrap(),
        ),
        super::CommitIdentity::new(
            IdentityName::try_from(name).unwrap(),
            EmailAddress::try_from(email).unwrap(),
        ),
    )
}

fn standard() -> &'static DeclaredIdentity {
    static STANDARD: std::sync::LazyLock<DeclaredIdentity> =
        std::sync::LazyLock::new(|| declared("Owner Example", "owner@client.example"));
    &STANDARD
}

fn finding(slot: Slot, scope: &str) -> Finding {
    let origin = ConfigOrigin::parse(format!("{scope}\tfile:/home/a/.gitconfig\tv").as_bytes());
    Finding::new(slot, Source::config(slot.role_key(), origin))
}

fn texts(lines: &[crate::Diagnostic]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn a_wrong_global_identity_is_named_by_layer_scope_and_file_with_the_fix() {
    let home = Home::new("/home/a");
    let user_email = Field::Email.user_key();
    let origin = ConfigOrigin::parse(b"global\tfile:/home/a/.gitconfig\twrong@example.invalid");
    let found = Finding::new(
        Slot::new(super::Role::Author, Field::Email),
        Source::config(user_email, origin),
    );
    let findings = Findings::new(vec![found]).unwrap();
    let report = Report::new(standard(), Some(&home));
    assert_eq!(
        texts(&report.finding_lines(Command::Check.level(), &findings, true)),
        [
            "error [identity] git takes the email (user.email) from global config (~/.gitconfig), \
which differs from the commit identity declared by layer 'client' (path:../client); \
fix: git config --local user.email 'owner@client.example'"
        ]
    );
}

#[test]
fn environment_command_and_host_sources_have_their_own_fix() {
    let author_email = Slot::new(super::Role::Author, Field::Email);
    let command = ConfigOrigin::parse(b"command\tcommand line:\tv");
    let findings = Findings::new(vec![
        Finding::new(author_email, Source::Environment(author_email)),
        Finding::new(
            Slot::new(super::Role::Committer, Field::Email),
            Source::config(Field::Email.user_key(), command),
        ),
        Finding::new(
            Slot::new(super::Role::Author, Field::Name),
            Source::Guessed(Field::Name),
        ),
    ])
    .unwrap();
    let lines = texts(&Report::new(standard(), None).finding_lines(
        Command::Build.level(),
        &findings,
        true,
    ));
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("warning [identity] git takes the name (user.name) from no setting (git guessed it from the host)"));
    assert!(lines[0].ends_with("fix: git config --local user.name 'Owner Example'"));
    assert!(lines[1].contains("from the environment (GIT_AUTHOR_EMAIL)"));
    assert!(lines[1].ends_with("fix: unset GIT_AUTHOR_EMAIL"));
    assert!(lines[2].contains("from command-line config (command line:)"));
    assert!(lines[2].ends_with("fix: unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS"));
}

#[test]
fn a_value_that_is_not_shell_safe_gets_a_sentence_and_no_command() {
    let report_for = declared("O'Example", "owner@client.example");
    let findings = Findings::new(vec![finding(
        Slot::new(super::Role::Author, Field::Name),
        "global",
    )])
    .unwrap();
    let lines = texts(&Report::new(&report_for, None).finding_lines(
        Command::Check.level(),
        &findings,
        true,
    ));
    assert!(lines[0].ends_with(
        "fix: set author.name to the value declared in layer 'client' (path:../client) \
(`alz build --adopt` adds it when unset)"
    ));
    assert!(!lines[0].contains("git config"));
}

#[test]
fn no_fix_is_printed_where_a_command_would_write_to_another_repository() {
    let findings = Findings::new(vec![finding(
        Slot::new(super::Role::Author, Field::Name),
        "global",
    )])
    .unwrap();
    let lines = texts(&Report::new(standard(), None).finding_lines(
        Command::Build.level(),
        &findings,
        false,
    ));
    assert!(!lines[0].contains("fix:"));
}

#[test]
fn state_lines_and_levels() {
    let report = Report::new(standard(), None);
    assert_eq!(
        texts(&report.check_lines(Command::Check, &IdentityCheck::NotARepository, true)),
        ["note [identity] this is not a git repository, so the commit identity was not checked"]
    );
    assert_eq!(
        texts(&report.check_lines(Command::Check, &IdentityCheck::Unavailable, true)),
        [
            "error [identity] git is not available or failed, so the commit identity could not be checked"
        ]
    );
    assert_eq!(
        texts(&report.check_lines(Command::Init, &IdentityCheck::Unavailable, true)),
        [
            "warning [identity] git is not available or failed, so the commit identity could not be checked"
        ]
    );
    assert_eq!(
        report.check_lines(Command::Check, &IdentityCheck::Matches, true),
        []
    );
}

#[test]
fn setup_lines_name_the_key_the_declared_value_and_the_layer() {
    let report = Report::new(standard(), None);
    assert_eq!(
        report.written_line(Field::Email).to_string(),
        "set user.email in .git/config to 'owner@client.example' (declared by layer 'client' (path:../client))"
    );
    assert_eq!(
        report.kept_line(Field::Name).to_string(),
        "warning [identity] user.name in .git/config differs from the identity declared by layer 'client' (path:../client); alz never replaces it. Fix: git config --local user.name 'Owner Example'"
    );
    assert_eq!(
        Report::unreadable_line(Field::Name).to_string(),
        "warning [identity] could not read .git/config; set user.name by hand"
    );
    assert_eq!(
        Report::write_failed_line(Field::Email).to_string(),
        "warning [identity] could not write user.email to .git/config; set it by hand"
    );
}

#[test]
fn a_blocked_place_prints_commands_by_hand_or_says_to_run_git_init() {
    let report = Report::new(standard(), None);
    let by_hand = report
        .blocked_line(super::Blocker::GitNotADirectory)
        .to_string();
    assert_eq!(
        by_hand,
        "warning [identity] .git is not a plain directory (a linked worktree, submodule or symlink), \
so alz did not write the git identity declared by layer 'client' (path:../client). \
Run by hand: git config --local user.name 'Owner Example'; git config --local user.email 'owner@client.example'"
    );
    let top = report.blocked_line(super::Blocker::NotTopLevel).to_string();
    assert!(
        top.ends_with("Run `git init` here first, or run alz from the repository's top level.")
    );
    assert!(!top.contains("git config"));
}

#[test]
fn the_home_folder_is_shown_as_a_tilde_only_at_a_folder_boundary() {
    let home = Home::new("/home/a");
    let shown = |path: &str| {
        home.display(&crate::Stripped::from_untrusted(path))
            .into_string()
    };
    assert_eq!(shown("/home/a/.gitconfig"), "~/.gitconfig");
    assert_eq!(shown("/home/a"), "~");
    assert_eq!(shown("/home/ab/.gitconfig"), "/home/ab/.gitconfig");
    let root = Home::new("/");
    assert_eq!(
        root.display(&crate::Stripped::from_untrusted("/etc/gitconfig"))
            .as_str(),
        "/etc/gitconfig"
    );
}

/// A wrong value no declared value, key, word or path can contain.
fn wrong_value() -> impl Strategy<Value = String> {
    "ZQ[0-9a-f]{10}"
}

proptest! {
    #[test]
    fn no_report_line_holds_a_value_git_reported(
        wrong_name in wrong_value(),
        wrong_email in wrong_value(),
        config_value in wrong_value(),
        scope in proptest::sample::select(vec!["system", "global", "local", "worktree", "command", "odd"]),
        env_set in any::<bool>(),
    ) {
        let declared = standard();
        let var = |value: &str| format!("{wrong_name} <{value}> 1700000000 +0000\n");
        let verdicts = Verdicts {
            author: Effective::compare(Some(var(&wrong_email).as_bytes()), declared.identity()),
            committer: Effective::compare(None, declared.identity()),
        };
        let presence = Presence::new(
            if env_set { vec![Slot::new(super::Role::Author, Field::Email)] } else { vec![] },
            false,
        );
        let line = format!("{scope}\tfile:/home/a/.gitconfig\t{config_value}");
        let check = IdentityCheck::decide(
            RepoState::Repository,
            || verdicts,
            |slot| locate(slot, &presence, |_| ConfigAnswer::from_git(Some(0), line.as_bytes())),
        );
        let home = Home::new("/home/a");
        let report = Report::new(declared, Some(&home));
        for command in [Command::Init, Command::Build, Command::Check] {
            for with_fix in [true, false] {
                for text in texts(&report.check_lines(command, &check, with_fix)) {
                    for value in [&wrong_name, &wrong_email, &config_value] {
                        prop_assert!(!text.contains(value.as_str()), "{text}");
                    }
                }
            }
        }
        let debug = format!("{check:?}");
        for value in [&wrong_name, &wrong_email, &config_value] {
            prop_assert!(!debug.contains(value.as_str()));
        }
    }

    #[test]
    fn a_fix_command_is_printed_exactly_when_the_declared_value_is_shell_safe(
        name in "[A-Za-z0-9][A-Za-z0-9 _+@éß'!$;\"#()-]{0,18}[A-Za-z0-9]",
        email in "[a-z][a-z0-9._%+-]{0,7}@[a-z]{2,6}\\.example",
    ) {
        let (Ok(name), Ok(email)) = (IdentityName::try_from(name.as_str()), EmailAddress::try_from(email.as_str())) else {
            return Err(proptest::test_runner::TestCaseError::reject("not a valid identity"));
        };
        let by = DeclaredBy::new(
            &LayerName::try_from("client").unwrap(),
            &StackSource::try_from("path:../client").unwrap(),
        );
        let declared = DeclaredIdentity::new(by, super::CommitIdentity::new(name.clone(), email.clone()));
        let findings = Findings::new(vec![
            finding(Slot::new(super::Role::Author, Field::Name), "global"),
            finding(Slot::new(super::Role::Author, Field::Email), "global"),
        ]).unwrap();
        let lines = texts(&Report::new(&declared, None).finding_lines(Command::Check.level(), &findings, true));
        for (line, (key, value)) in lines.iter().zip([("author.name", name.as_str()), ("author.email", email.as_str())]) {
            let command = format!("git config --local {key} '{value}'");
            let safe = ShellSafe::try_from(value).is_ok();
            prop_assert_eq!(line.contains(&command), safe, "{}", line);
            prop_assert_eq!(line.contains(&format!("set {key} to the value declared in")), !safe);
        }
    }

    #[test]
    fn findings_print_name_before_email_and_author_before_committer_whatever_the_input_order(
        order in Just(Slot::all().to_vec()).prop_shuffle(),
        repeat in 0usize..3,
    ) {
        let found: Vec<Finding> = order
            .iter()
            .flat_map(|slot| std::iter::repeat_n(finding(*slot, "global"), repeat + 1))
            .collect();
        let findings = Findings::new(found).unwrap();
        let lines = texts(&Report::new(standard(), None).finding_lines(Command::Build.level(), &findings, true));
        let keys: Vec<&str> = lines
            .iter()
            .map(|line| line.split(['(', ')']).nth(1).unwrap_or_default())
            .collect();
        prop_assert_eq!(keys, ["author.name", "committer.name", "author.email", "committer.email"]);
    }

    #[test]
    fn a_setup_plan_adds_exactly_the_keys_that_are_missing(
        name in key_state(),
        email in key_state(),
    ) {
        let plan = SetupPlan::new(name, email);
        for (field, state) in [(Field::Name, name), (Field::Email, email)] {
            prop_assert_eq!(
                plan.additions().any(|added| added == field),
                state == KeyState::Missing
            );
        }
    }

    #[test]
    fn the_parsers_are_total_over_arbitrary_bytes(
        bytes in any::<Vec<u8>>(),
        code in proptest::option::of(any::<i32>()),
    ) {
        let declared = standard();
        black_box(Effective::compare(Some(&bytes), declared.identity()));
        black_box(ConfigOrigin::parse(&bytes));
        black_box(ConfigAnswer::from_git(code, &bytes));
        black_box(KeyState::from_get(code, &bytes, "Owner Example"));
        black_box(RepoState::classify(code, &bytes));
    }
}

fn key_state() -> impl Strategy<Value = KeyState> {
    proptest::sample::select(vec![
        KeyState::Missing,
        KeyState::Equal,
        KeyState::Different,
        KeyState::Unreadable,
    ])
}
