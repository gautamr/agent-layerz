use std::cell::Cell;

use proptest::prelude::{any, prop_assert, proptest};

use super::which_report;
use crate::account::resolution::{
    AllowedFact, ConfigFact, EnvironmentFact, ExistingFolders, Explicit, ExplicitName, Facts,
    FileFact, FileOutcome, HomeFact, ManifestFact, NoRepository, Repository, Stack, VariableFact,
    resolve,
};
use crate::account::{
    AccountChoice, AccountName, AllowEntry, AllowedSource, CachedDeclaration, Miss, MissReason,
    NonEmptyTargetMap,
};
use crate::diagnostic::{Diagnostics, Message};
use crate::path::AbsolutePath;
use crate::target::TargetName;
use crate::untrusted_text::{Stripped, has_control_or_format};

struct Fixed(CachedDeclaration, Cell<usize>);

impl Stack for Fixed {
    fn declaration(&self) -> CachedDeclaration {
        self.1.set(self.1.get() + 1);
        self.0.clone()
    }

    fn contains(&self, _recorded: &AllowedSource) -> bool {
        true
    }
}

fn path(text: &str) -> AbsolutePath {
    AbsolutePath::try_from(text).expect("an absolute path")
}

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("an account name")
}

fn file(shown: &str, outcome: FileOutcome) -> FileFact {
    FileFact {
        shown: Stripped::from_untrusted(shown),
        outcome,
    }
}

fn base() -> Facts {
    Facts {
        home: HomeFact::Absolute {
            given: path("/home/u"),
            real: None,
        },
        environment: EnvironmentFact {
            claude: VariableFact::Unset,
            codex: VariableFact::Unset,
            manager_active: false,
        },
        explicit: Explicit::default(),
        config: ConfigFact {
            file: file(
                "~/.config/agent-layerz/accounts.yaml",
                FileOutcome::NotFound,
            ),
            default_name: None,
            folders: Vec::new(),
        },
        allowed: AllowedFact {
            file: file(
                "~/.local/state/agent-layerz/allowed.yaml",
                FileOutcome::NotFound,
            ),
            entries: Vec::new(),
            unresolved: Vec::new(),
        },
        repository: Repository::None(NoRepository::NotFound),
        known_directory: None,
        folders: ExistingFolders::default(),
    }
}

fn nothing() -> Fixed {
    Fixed(
        CachedDeclaration::None {
            ignored: Vec::new(),
        },
        Cell::new(0),
    )
}

fn report(facts: &Facts, stack: &Fixed) -> (Vec<String>, u8) {
    let result = resolve(facts, stack);
    let report = which_report(facts, &result, Diagnostics::new());
    (
        report.lines().iter().map(ToString::to_string).collect(),
        report.exit_code(),
    )
}

#[test]
fn the_default_report_is_the_text_of_the_spec_and_exits_0() {
    let (lines, exit) = report(&base(), &nothing());
    assert_eq!(
        lines,
        [
            "repository: none (no repository here)",
            "accounts.yaml: ~/.config/agent-layerz/accounts.yaml (not found)",
            "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (not found)",
            "claude: default (source: default) CLAUDE_CONFIG_DIR unset",
            "  use step skipped (no alz use choice)",
            "  user config step skipped (no accounts.yaml)",
            "  layer step skipped (no repository here)",
            "codex: default (source: default) CODEX_HOME unset",
            "  use step skipped (no alz use choice)",
            "  user config step skipped (no accounts.yaml)",
            "  layer step skipped (no repository here)",
        ]
    );
    assert_eq!(exit, 0);
}

#[test]
fn a_missing_folder_reports_the_text_of_the_spec_and_exits_1() {
    let mut facts = base();
    facts.folders = ExistingFolders::new([name("a"), name("b")], []);
    facts.explicit.used = vec![(
        TargetName::Claude,
        ExplicitName::Valid(AccountChoice::Named(name("nope"))),
    )];
    facts.explicit.run = Some((
        TargetName::Codex,
        ExplicitName::Valid(AccountChoice::Named(name("alz-blocked"))),
    ));
    let (lines, exit) = report(&facts, &nothing());
    assert_eq!(exit, 1);
    assert!(lines.contains(&"claude: fails closed (source: use)".to_owned()));
    assert!(lines.contains(
        &"  use step names nope, but ~/.claude-nope does not exist; fails closed".to_owned()
    ));
    assert!(lines.contains(
        &"  the hook applies CLAUDE_CONFIG_DIR=~/.claude-nope (the missing folder)".to_owned()
    ));
    assert!(
        lines.contains(&"  fix: alz account init nope (account folders here: a, b)".to_owned())
    );
    assert!(lines.contains(&"  fix: the name alz-blocked is reserved for the blocked folders; choose another account name".to_owned()));
}

#[test]
fn a_cache_miss_names_the_recorded_account_and_asks_for_a_build() {
    let mut facts = base();
    facts.repository = Repository::Root {
        path: path("/work/a"),
        manifest: ManifestFact::Read,
    };
    facts.allowed.file.outcome = FileOutcome::Read;
    facts.allowed.entries = vec![AllowEntry::new(
        path("/work/a"),
        NonEmptyTargetMap::from_entries([
            (TargetName::Claude, name("client")),
            (TargetName::Codex, name("default")),
        ])
        .expect("a map"),
        AllowedSource::try_from("/s").expect("a source"),
    )];
    facts.folders = ExistingFolders::new([], []);
    let stack = Fixed(
        CachedDeclaration::Miss(Miss::of(MissReason::LockMissing)),
        Cell::new(0),
    );
    let (lines, exit) = report(&facts, &stack);
    assert_eq!(exit, 1);
    for expected in [
        "repository: /work/a",
        "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (read; this repository is allowed)",
        "claude: fails closed (source: layer)",
        "  layer step skipped (cache miss: layer.lock is missing); fails closed",
        "  the hook applies CLAUDE_CONFIG_DIR=~/.claude-client (the recorded account client; that folder does not exist)",
        "  the hook applies CODEX_HOME unset (the recorded account default)",
        "  fix: alz build",
    ] {
        assert!(
            lines.contains(&expected.to_owned()),
            "{expected}\n{lines:#?}"
        );
    }
}

#[test]
fn a_changed_declaration_and_a_removed_one_name_their_fix() {
    let mut facts = base();
    facts.repository = Repository::Root {
        path: path("/work/a"),
        manifest: ManifestFact::Unreadable,
    };
    facts.allowed.file.outcome = FileOutcome::Read;
    facts.allowed.entries = vec![AllowEntry::new(
        path("/work/a"),
        NonEmptyTargetMap::from_entries([(TargetName::Claude, name("client"))]).expect("a map"),
        AllowedSource::try_from("/s").expect("a source"),
    )];
    let (lines, _) = report(&facts, &nothing());
    for expected in [
        "repository: /work/a (layer.yaml cannot be read)",
        "  layer step skipped (layer.yaml cannot be read); fails closed",
        "  the hook applies CLAUDE_CONFIG_DIR=~/.claude-alz-blocked (the blocked folder)",
        "  fix: repair /work/a/layer.yaml",
    ] {
        assert!(
            lines.contains(&expected.to_owned()),
            "{expected}\n{lines:#?}"
        );
    }
    facts.repository = Repository::Root {
        path: path("/work/a"),
        manifest: ManifestFact::Read,
    };
    let (lines, _) = report(&facts, &nothing());
    assert!(lines.contains(
        &"  layer step skipped (the layers no longer declare an account); fails closed".to_owned()
    ));
    assert!(lines.contains(&"  fix: alz account allow --remove".to_owned()));
}

#[test]
fn a_value_set_outside_alz_is_shown_stripped_and_kept() {
    let mut facts = base();
    facts.environment.codex = VariableFact::Set {
        value: String::new(),
        marker: None,
    };
    facts.environment.claude = VariableFact::Set {
        value: "/home/u/x\u{1b}[31m".to_owned(),
        marker: None,
    };
    let (lines, exit) = report(&facts, &nothing());
    assert_eq!(exit, 0);
    assert!(
        lines.contains(
            &"claude: set outside alz (source: environment) CLAUDE_CONFIG_DIR=\"~/x[31m\" kept"
                .to_owned()
        )
    );
    assert!(
        lines.contains(
            &"codex: set outside alz (source: environment) CODEX_HOME=\"\" kept".to_owned()
        )
    );
}

#[test]
fn a_stale_approval_is_quoted_with_a_tilde_for_home() {
    let mut facts = base();
    facts.allowed.file.outcome = FileOutcome::Read;
    facts.allowed.unresolved = vec![path("/home/u/we\u{1b}ird"), path("/home/u/gone")];
    let (lines, _) = report(&facts, &nothing());
    assert!(lines.contains(&"  stale approval: \"~/gone\" does not exist here".to_owned()));
    assert!(lines.contains(&"  stale approval: \"~/weird\" does not exist here".to_owned()));
}

#[test]
fn an_ignored_file_says_why_and_the_default_name_is_called() {
    let mut facts = base();
    facts.config.file.outcome = FileOutcome::Ignored(Message::new("it is not valid YAML"));
    facts.config.default_name = Some(name("personal"));
    let (lines, _) = report(&facts, &nothing());
    assert!(
        lines.contains(
            &"accounts.yaml: ~/.config/agent-layerz/accounts.yaml (ignored: it is not valid YAML)"
                .to_owned()
        )
    );
    assert!(
        lines.contains(
            &"claude: default, called personal here (source: default) CLAUDE_CONFIG_DIR unset"
                .to_owned()
        )
    );
    assert!(lines.contains(&"  user config step skipped (accounts.yaml is ignored)".to_owned()));
}

proptest! {
    /// No report line, and no warning, holds a control or format character, whatever the
    /// environment values and the paths hold; and an explicit name that is not an account name is
    /// never repeated.
    #[test]
    fn no_line_holds_a_control_or_format_character(
        value in any::<String>(),
        shown in any::<String>(),
        root in "/[a-z\u{1}-\u{1f}\u{200b}\u{202e}/ ]{0,12}",
        stale in "/[a-z\u{1}-\u{1f}\u{200b}\u{85}\u{2028}/ ]{0,12}",
    ) {
        let mut facts = base();
        facts.environment.claude = VariableFact::Set { value, marker: None };
        facts.config.file = file(&shown, FileOutcome::Ignored(Message::new("it is not valid YAML")));
        if let Ok(root) = AbsolutePath::try_from(root.as_str()) {
            facts.repository = Repository::Root { path: root, manifest: ManifestFact::Invalid };
        }
        if let Ok(stale) = AbsolutePath::try_from(stale.as_str()) {
            facts.allowed.unresolved = vec![stale];
        }
        facts.explicit.used = vec![(TargetName::Codex, ExplicitName::NotAName)];
        let result = resolve(&facts, &nothing());
        let report = which_report(&facts, &result, Diagnostics::new());
        for line in report.lines() {
            let text = line.to_string();
            prop_assert!(!has_control_or_format(&text), "{text:?}");
        }
        for warning in report.warnings().iter() {
            let text = warning.to_string();
            prop_assert!(!has_control_or_format(&text), "{text:?}");
        }
    }
}
