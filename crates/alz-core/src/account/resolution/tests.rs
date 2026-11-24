use std::cell::Cell;

use proptest::prelude::{Just, Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest};

use super::{
    AllowedFact, Applied, ConfigFact, EnvironmentFact, ExistingFolders, Explicit, ExplicitName,
    Facts, FailsClosed, FileFact, FileOutcome, HomeFact, HookApplies, ManifestFact,
    ManifestProblem, NoRepository, Recorded, Repository, SkipReason, Source, Stack, VariableFact,
    Verdict, resolve,
};
use crate::account::CachedLayer;
use crate::account::{
    AccountChoice, AccountDeclaration, AccountName, AllowEntry, AllowedSource, CachedDeclaration,
    Miss, MissReason, NonEmptyTargetMap, decide,
};
use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::merge::Placement;
use crate::path::AbsolutePath;
use crate::stack::{ResolvedLayer, Source as StackSource};
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

fn name(text: &str) -> AccountName {
    AccountName::try_from(text).expect("an account name")
}

fn choice(text: &str) -> AccountChoice {
    AccountChoice::try_from(text).expect("a choice")
}

fn path(text: &str) -> AbsolutePath {
    AbsolutePath::try_from(text).expect("an absolute path")
}

fn file(outcome: FileOutcome) -> FileFact {
    FileFact {
        shown: Stripped::from_untrusted("~/f.yaml"),
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
            file: file(FileOutcome::NotFound),
            default_name: None,
            folders: Vec::new(),
        },
        allowed: AllowedFact {
            file: file(FileOutcome::NotFound),
            entries: Vec::new(),
            unresolved: Vec::new(),
        },
        repository: Repository::None(NoRepository::NotFound),
        known_directory: Some(path("/work/a/b")),
        folders: ExistingFolders::default(),
    }
}

/// Stack with a fixed answer that counts how often the layers are read.
struct Fixed {
    declaration: CachedDeclaration,
    in_stack: bool,
    reads: Cell<usize>,
}

impl Fixed {
    fn new(declaration: CachedDeclaration) -> Self {
        Self {
            declaration,
            in_stack: true,
            reads: Cell::new(0),
        }
    }
}

impl Stack for Fixed {
    fn declaration(&self) -> CachedDeclaration {
        self.reads.set(self.reads.get() + 1);
        self.declaration.clone()
    }

    fn contains(&self, _recorded: &AllowedSource) -> bool {
        self.in_stack
    }
}

fn nothing() -> Fixed {
    Fixed::new(CachedDeclaration::None {
        ignored: Vec::new(),
    })
}

/// What a first layer declaring `account` (YAML) gives, recorded from `recorded`.
fn declares(account: &str, recorded: &str) -> CachedDeclaration {
    let yaml = format!("name: first\nkind: personal\nvisibility: private\naccount: {account}\n");
    let node = parse_document(yaml.as_bytes()).expect("valid YAML");
    let meta = LayerMeta::from_yaml(&node).expect("a layer");
    decide(&[CachedLayer {
        position: 0,
        placement: Placement::Outside,
        layer: ResolvedLayer::new(
            StackSource::try_from("path:../first").expect("a source"),
            meta,
            LayerContents::default(),
            None,
        ),
        git_ref: None,
        revision: None,
        recorded_source: recorded.to_owned(),
    }])
}

fn entry(root: &str, claude: Option<&str>, codex: Option<&str>, source: &str) -> AllowEntry {
    let map = NonEmptyTargetMap::from_entries(
        [(TargetName::Claude, claude), (TargetName::Codex, codex)]
            .into_iter()
            .filter_map(|(target, account)| account.map(|text| (target, name(text)))),
    )
    .expect("a non-empty map");
    AllowEntry::new(
        path(root),
        map,
        AllowedSource::try_from(source).expect("a source"),
    )
}

/// An allowed repository at `/work/a` that records `entry`, with `~/.claude-client` present.
fn allowed(entries: Vec<AllowEntry>) -> Facts {
    Facts {
        repository: Repository::Root {
            path: path("/work/a"),
            manifest: ManifestFact::Read,
        },
        allowed: AllowedFact {
            file: file(FileOutcome::Read),
            entries,
            unresolved: Vec::new(),
        },
        folders: ExistingFolders::new([name("client")], [name("client")]),
        ..base()
    }
}

fn verdict(facts: &Facts, stack: &Fixed, target: TargetName) -> Verdict {
    resolve(facts, stack)
        .target(target)
        .expect("every target is decided")
        .verdict
        .clone()
}

fn source_of(facts: &Facts, stack: &Fixed, target: TargetName) -> Source {
    resolve(facts, stack)
        .target(target)
        .expect("every target is decided")
        .source
}

#[test]
fn nothing_named_is_the_default_account_and_never_fails() {
    let result = resolve(&base(), &nothing());
    assert!(result.all_resolve());
    for (_, found) in result.targets() {
        assert_eq!(found.source, Source::Default);
        assert_eq!(found.verdict, Verdict::Applied(Applied::Default));
    }
}

#[test]
fn a_named_account_whose_folder_exists_is_applied_and_a_missing_one_fails_closed() {
    let mut facts = base();
    facts.explicit.run = Some((TargetName::Claude, ExplicitName::Valid(choice("client"))));
    facts.folders = ExistingFolders::new([name("other")], []);
    let Verdict::FailsClosed(FailsClosed::MissingFolder {
        name: missing,
        existing,
    }) = verdict(&facts, &nothing(), TargetName::Claude)
    else {
        panic!("a missing folder fails closed");
    };
    assert_eq!(missing, name("client"));
    assert_eq!(existing, vec![name("other")]);
    facts.folders = ExistingFolders::new([name("client")], []);
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::Applied(Applied::Folder(name("client")))
    );
}

#[test]
fn the_blocked_name_never_exists_so_it_is_a_missing_folder() {
    let mut facts = base();
    facts.folders = ExistingFolders::new([name("alz-blocked")], []);
    facts.explicit.used = vec![(
        TargetName::Claude,
        ExplicitName::Valid(choice("alz-blocked")),
    )];
    let found = verdict(&facts, &nothing(), TargetName::Claude);
    assert!(matches!(
        &found,
        Verdict::FailsClosed(failure @ FailsClosed::MissingFolder { .. })
            if failure.fix() == super::Fix::ReservedName
    ));
}

#[test]
fn an_explicit_name_that_is_not_an_account_name_fails_closed_and_never_falls_through() {
    let mut facts = base();
    facts.explicit.used = vec![(TargetName::Codex, ExplicitName::NotAName)];
    facts.config.file = file(FileOutcome::Read);
    facts.config.folders = vec![(path("/work"), AccountDeclaration::Every(choice("client")))];
    facts.folders = ExistingFolders::new([name("client")], [name("client")]);
    let result = resolve(&facts, &nothing());
    let found = result.target(TargetName::Codex).expect("decided");
    assert_eq!(found.source, Source::Use);
    assert_eq!(
        found.verdict,
        Verdict::FailsClosed(FailsClosed::InvalidName)
    );
    assert_eq!(
        found.verdict,
        Verdict::FailsClosed(FailsClosed::InvalidName)
    );
    assert_eq!(
        result.target(TargetName::Claude).expect("decided").source,
        Source::UserConfig
    );
}

#[test]
fn with_home_not_absolute_a_named_account_fails_closed_and_the_default_still_resolves() {
    let mut facts = base();
    facts.home = HomeFact::NotAbsolute;
    facts.explicit.run = Some((TargetName::Claude, ExplicitName::Valid(choice("client"))));
    facts.explicit.used = vec![(
        TargetName::Codex,
        ExplicitName::Valid(AccountChoice::Default),
    )];
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::HomeNotAbsolute)
    );
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Codex),
        Verdict::Applied(Applied::Default)
    );
}

#[test]
fn a_name_equal_to_the_default_name_is_the_default_account() {
    let mut facts = base();
    facts.config.default_name = Some(name("personal"));
    facts.explicit.run = Some((TargetName::Claude, ExplicitName::Valid(choice("personal"))));
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::Applied(Applied::Default)
    );
}

#[test]
fn a_default_name_that_is_also_an_account_folder_warns_with_the_spec_text() {
    let mut facts = base();
    facts.config.default_name = Some(name("personal"));
    facts.folders = ExistingFolders::new([], [name("personal")]);
    let warnings: Vec<String> = resolve(&facts, &nothing())
        .warnings()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        warnings,
        [
            "warning [account-default-name-folder] accounts.yaml: default_name 'personal' is also the name of an account folder (~/.claude-personal or ~/.codex-personal); that folder can never be chosen"
        ]
    );
}

#[test]
fn a_value_set_outside_alz_is_foreign_unless_its_marker_holds_the_same_text() {
    let set = |value: &str, marker: Option<&str>| VariableFact::Set {
        value: value.to_owned(),
        marker: marker.map(str::to_owned),
    };
    assert_eq!(VariableFact::Unset.foreign(), None);
    assert_eq!(set("/x", Some("/x")).foreign(), None);
    assert_eq!(set("/x", Some("/y")).foreign(), Some("/x"));
    assert_eq!(set("/x", None).foreign(), Some("/x"));
    assert_eq!(set("", None).foreign(), Some(""));
    assert_eq!(set("", Some("")).foreign(), None);
}

#[test]
fn a_foreign_value_replaces_steps_three_to_five_but_never_one_and_two() {
    let mut facts = allowed(vec![entry("/work/a", Some("client"), Some("client"), "/s")]);
    facts.environment.claude = VariableFact::Set {
        value: "/elsewhere".to_owned(),
        marker: None,
    };
    facts.environment.manager_active = true;
    facts.explicit.used = vec![(TargetName::Claude, ExplicitName::Valid(choice("client")))];
    let stack = Fixed::new(declares("client", "/s"));
    assert_eq!(source_of(&facts, &stack, TargetName::Claude), Source::Use);
    facts.explicit.used.clear();
    facts.environment.codex = VariableFact::Set {
        value: "/elsewhere".to_owned(),
        marker: Some("/other".to_owned()),
    };
    let stack = Fixed::new(declares("client", "/s"));
    let result = resolve(&facts, &stack);
    let found = result.target(TargetName::Claude).expect("decided");
    assert_eq!(found.source, Source::Environment);
    assert!(matches!(found.verdict, Verdict::Applied(Applied::Kept(_))));
    assert_eq!(
        stack.reads.get(),
        0,
        "step 4 is never reached for the foreign target"
    );
    let warnings: Vec<String> = result.warnings().iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "warning [account-foreign] CLAUDE_CONFIG_DIR was set outside alz; mise or direnv is active here; alz keeps it",
            "warning [account-foreign] CODEX_HOME was set outside alz; mise or direnv is active here; alz keeps it"
        ]
    );
}

#[test]
fn a_marker_without_its_variable_is_stale_and_ignored() {
    let facts = base();
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::Applied(Applied::Default)
    );
}

fn with_folders(entries: Vec<(&str, AccountDeclaration)>) -> Facts {
    Facts {
        config: ConfigFact {
            file: file(FileOutcome::Read),
            default_name: None,
            folders: entries
                .into_iter()
                .map(|(folder, declaration)| (path(folder), declaration))
                .collect(),
        },
        folders: ExistingFolders::new([name("a"), name("b")], [name("a"), name("b")]),
        ..base()
    }
}

#[test]
fn the_longest_folders_entry_decides_and_a_shorter_one_is_not_tried() {
    let only_codex = AccountDeclaration::PerTarget(
        NonEmptyTargetMap::from_entries([(TargetName::Codex, choice("b"))]).expect("a map"),
    );
    let facts = with_folders(vec![
        ("/work", AccountDeclaration::Every(choice("a"))),
        ("/work/a", only_codex),
    ]);
    let result = resolve(&facts, &nothing());
    let claude = result.target(TargetName::Claude).expect("decided");
    assert_eq!(
        claude.source,
        Source::Default,
        "the shorter entry is not tried"
    );
    assert!(
        claude
            .skipped
            .iter()
            .any(|skip| skip.reason == SkipReason::EntryNamesNoTarget)
    );
    assert_eq!(
        result.target(TargetName::Codex).expect("decided").verdict,
        Verdict::Applied(Applied::Folder(name("b")))
    );
}

#[test]
fn a_folders_entry_matches_whole_segments_only() {
    let facts = with_folders(vec![("/work/a", AccountDeclaration::Every(choice("a")))]);
    let mut near = facts.clone();
    near.known_directory = Some(path("/work/ab"));
    assert_eq!(
        source_of(&near, &nothing(), TargetName::Claude),
        Source::Default
    );
    assert_eq!(
        source_of(&facts, &nothing(), TargetName::Claude),
        Source::UserConfig
    );
}

#[test]
fn a_working_directory_in_an_account_folder_skips_the_user_config() {
    let mut facts = with_folders(vec![("/work", AccountDeclaration::Every(choice("a")))]);
    facts.repository = Repository::None(NoRepository::AccountFolder);
    let result = resolve(&facts, &nothing());
    let found = result.target(TargetName::Claude).expect("decided");
    assert_eq!(found.source, Source::Default);
    assert_eq!(found.skipped[1].reason, SkipReason::CwdInAccountFolder);
}

#[test]
fn a_repository_that_is_not_allowed_never_reads_the_layers_and_gets_the_default() {
    let facts = allowed(vec![entry("/work/other", Some("client"), None, "/s")]);
    let stack = Fixed::new(declares("client", "/s"));
    let result = resolve(&facts, &stack);
    assert_eq!(stack.reads.get(), 0);
    assert!(result.all_resolve());
    assert!(!result.is_allowed());
    let found = result.target(TargetName::Claude).expect("decided");
    assert_eq!(
        found.skipped.last().map(|skip| skip.reason),
        Some(SkipReason::NotAllowed)
    );
}

#[test]
fn an_allowed_declaration_that_matches_the_entry_applies() {
    let facts = allowed(vec![entry("/work/a", Some("client"), None, "/s")]);
    let stack = Fixed::new(declares("{claude: client}", "/s"));
    assert_eq!(
        verdict(&facts, &stack, TargetName::Claude),
        Verdict::Applied(Applied::Folder(name("client")))
    );
    assert_eq!(
        stack.reads.get(),
        1,
        "the layers are read once for both targets"
    );
}

#[test]
fn a_changed_map_or_source_skips_step_four_for_every_target_and_warns() {
    let facts = allowed(vec![entry("/work/a", Some("client"), Some("client"), "/s")]);
    for (account, recorded) in [("{claude: client}", "/s"), ("client", "/other")] {
        let stack = Fixed::new(declares(account, recorded));
        let result = resolve(&facts, &stack);
        for (_, found) in result.targets() {
            assert_eq!(
                found.verdict,
                Verdict::FailsClosed(FailsClosed::Changed {
                    still_declared: true
                })
            );
        }
        assert_eq!(result.warnings().iter().count(), 1);
    }
    let warning = resolve(&facts, &Fixed::new(declares("{claude: other}", "/t")))
        .warnings()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        warning,
        [
            "warning [account-changed] the account declaration changed since it was allowed: was claude=client, codex=client from \"/s\", now claude=other from \"/t\""
        ]
    );
}

#[test]
fn a_declaration_that_is_gone_fails_closed_with_the_remove_fix() {
    let facts = allowed(vec![entry("/work/a", Some("client"), None, "/s")]);
    let Verdict::FailsClosed(failure) = verdict(&facts, &nothing(), TargetName::Claude) else {
        panic!("fails closed");
    };
    assert_eq!(failure.fix(), super::Fix::AllowRemove);
    assert_eq!(failure.hook(), HookApplies::Blocked);
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Codex),
        Verdict::Applied(Applied::Default),
        "a target the entry does not name is skipped"
    );
}

#[test]
fn a_cache_miss_keeps_the_recorded_account_and_a_gone_source_blocks() {
    let facts = allowed(vec![entry("/work/a", Some("client"), Some("ghost"), "/s")]);
    let miss = CachedDeclaration::Miss(Miss::of(MissReason::LockMissing));
    let mut stack = Fixed::new(miss);
    assert_eq!(
        verdict(&facts, &stack, TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::CacheMiss {
            miss: MissReason::LockMissing,
            recorded: Recorded::Folder {
                name: name("client"),
                exists: true
            }
        })
    );
    assert_eq!(
        verdict(&facts, &stack, TargetName::Codex),
        Verdict::FailsClosed(FailsClosed::CacheMiss {
            miss: MissReason::LockMissing,
            recorded: Recorded::Folder {
                name: name("ghost"),
                exists: false
            }
        })
    );
    stack.in_stack = false;
    assert_eq!(
        verdict(&facts, &stack, TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::SourceGone)
    );
}

#[test]
fn a_recorded_default_account_is_unset_by_the_hook_and_still_fails_closed() {
    let facts = allowed(vec![entry("/work/a", Some("default"), None, "/s")]);
    let stack = Fixed::new(CachedDeclaration::Miss(Miss::of(MissReason::Unreadable)));
    let Verdict::FailsClosed(failure) = verdict(&facts, &stack, TargetName::Claude) else {
        panic!("fails closed");
    };
    assert_eq!(failure.hook(), HookApplies::Recorded(Recorded::Default));
}

#[test]
fn a_manifest_that_cannot_be_used_blocks_an_allowed_root_without_reading_the_layers() {
    let mut facts = allowed(vec![entry("/work/a", Some("client"), None, "/s")]);
    facts.repository = Repository::Root {
        path: path("/work/a"),
        manifest: ManifestFact::Unreadable,
    };
    let stack = nothing();
    assert_eq!(
        verdict(&facts, &stack, TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::Manifest(ManifestProblem::Unreadable))
    );
    assert_eq!(stack.reads.get(), 0);
}

#[test]
fn an_allowed_entry_covering_a_configured_folder_fails_closed_on_the_blocked_folder() {
    let mut facts = allowed(vec![entry("/work", Some("client"), None, "/s")]);
    facts.repository = Repository::None(NoRepository::ConfiguredFolder);
    facts.known_directory = Some(path("/work/cfg"));
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::ConfiguredFolder)
    );
    facts.allowed.entries.clear();
    let result = resolve(&facts, &nothing());
    assert!(result.all_resolve());
    let found = result.target(TargetName::Claude).expect("decided");
    assert_eq!(
        found.skipped.last().map(|skip| skip.reason),
        Some(SkipReason::CwdInConfiguredFolder)
    );
}

#[test]
fn an_allowed_entry_inside_a_configured_folder_fails_closed_and_one_beside_it_does_not() {
    // The working directory is known only as the configured folder, so a root inside it may be
    // where the person is; a root elsewhere is not.
    let mut facts = allowed(vec![entry("/work/cfg/repo", Some("client"), None, "/s")]);
    facts.repository = Repository::None(NoRepository::ConfiguredFolder);
    facts.known_directory = Some(path("/work/cfg"));
    assert_eq!(
        verdict(&facts, &nothing(), TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::ConfiguredFolder)
    );
    facts.allowed.entries = vec![entry("/work/other", Some("client"), None, "/s")];
    let result = resolve(&facts, &nothing());
    assert!(result.all_resolve());
}

#[test]
fn a_conflict_blocks_and_reports_the_declarations_that_may_not() {
    let facts = allowed(vec![entry("/work/a", Some("client"), None, "/s")]);
    let stack = Fixed::new(CachedDeclaration::Conflict {
        clients: Vec::new(),
        ignored: Vec::new(),
    });
    assert_eq!(
        verdict(&facts, &stack, TargetName::Claude),
        Verdict::FailsClosed(FailsClosed::Conflict)
    );
}

fn some_name() -> impl Strategy<Value = AccountName> {
    prop_oneof!["client", "other", "personal", "x-1"].prop_map(|text: String| name(&text))
}

fn some_choice() -> impl Strategy<Value = AccountChoice> {
    prop_oneof![
        Just(AccountChoice::Default),
        some_name().prop_map(AccountChoice::Named)
    ]
}

fn some_declaration() -> impl Strategy<Value = CachedDeclaration> {
    prop_oneof![
        Just(CachedDeclaration::Miss(Miss::of(MissReason::IndexMissing))),
        Just(CachedDeclaration::None {
            ignored: Vec::new()
        }),
        Just(CachedDeclaration::Conflict {
            clients: Vec::new(),
            ignored: Vec::new()
        }),
        Just(declares("other", "/s")),
        Just(declares("{claude: client}", "/s")),
        Just(declares("client", "/t")),
        Just(declares("client", "/s")),
    ]
}

fn some_existing() -> impl Strategy<Value = ExistingFolders> {
    (
        proptest::collection::vec(some_name(), 0..4),
        proptest::collection::vec(some_name(), 0..4),
    )
        .prop_map(|(claude, codex)| ExistingFolders::new(claude, codex))
}

proptest! {
    /// A named account that cannot apply is never the default account and never a newly declared
    /// one: for every target the entry names and a declaration that is not the recorded one, the
    /// verdict is a failure (or the recorded account's own folder), never `Default` or another
    /// account's folder.
    #[test]
    fn a_named_account_that_cannot_apply_never_resolves_to_the_default_or_a_new_account(
        declaration in some_declaration(),
        claude in some_name(),
        codex in proptest::option::of(some_name()),
        in_stack in any::<bool>(),
        existing in some_existing(),
    ) {
        let mut facts = allowed(vec![entry(
            "/work/a",
            Some(claude.as_str()),
            codex.as_ref().map(AccountName::as_str),
            "/s",
        )]);
        facts.folders = existing;
        let applies = matches!(&declaration, CachedDeclaration::Declared { declaration: d, .. }
            if d.recorded_source == "/s"
                && d.map.choice_for(TargetName::Claude).is_some_and(|c| *c == AccountChoice::Named(claude.clone()))
                && d.map.choice_for(TargetName::Codex).map(|c| match c {
                    AccountChoice::Default => "default".to_owned(),
                    AccountChoice::Named(n) => n.to_string(),
                }) == codex.as_ref().map(ToString::to_string));
        let stack = Fixed { in_stack, ..Fixed::new(declaration) };
        let result = resolve(&facts, &stack);
        for (target, recorded) in [(TargetName::Claude, Some(claude.clone())), (TargetName::Codex, codex.clone())] {
            let Some(recorded) = recorded else { continue };
            let found = result.target(target).expect("decided");
            if applies {
                continue;
            }
            // The only way to apply is the recorded account itself.
            match &found.verdict {
                Verdict::FailsClosed(_) => {}
                Verdict::Applied(Applied::Folder(_) | Applied::Default | Applied::Kept(_)) => {
                    prop_assert!(false, "{target:?} {recorded}: {found:?}");
                }
            }
            prop_assert_eq!(found.source, Source::Layer);
        }
    }

    /// The first step that names an account wins; a foreign value replaces steps 3 to 5 and never
    /// 1 and 2; a target the allowed entry does not name is skipped without failing.
    #[test]
    fn the_first_step_that_names_an_account_wins(
        run in proptest::option::of(some_choice()),
        used in proptest::option::of(some_choice()),
        foreign in any::<bool>(),
        config in proptest::option::of(some_choice()),
        declaration in some_declaration(),
        existing in some_existing(),
    ) {
        let mut facts = allowed(vec![entry("/work/a", Some("client"), None, "/s")]);
        facts.folders = existing;
        facts.explicit.run = run.clone().map(|c| (TargetName::Claude, ExplicitName::Valid(c)));
        facts.explicit.used = used.clone().into_iter().map(|c| (TargetName::Claude, ExplicitName::Valid(c))).collect();
        if foreign {
            facts.environment.claude = VariableFact::Set { value: "/x".to_owned(), marker: None };
        }
        facts.config.file = file(FileOutcome::Read);
        facts.config.folders = config.clone().into_iter().map(|c| (path("/work"), AccountDeclaration::Every(c))).collect();
        let result = resolve(&facts, &Fixed::new(declaration));
        let expected = if run.is_some() {
            Source::Run
        } else if used.is_some() {
            Source::Use
        } else if foreign {
            Source::Environment
        } else if config.is_some() {
            Source::UserConfig
        } else {
            let found = result.target(TargetName::Claude).expect("decided");
            prop_assert!(matches!(found.source, Source::Layer | Source::Default));
            Source::Layer
        };
        let found = result.target(TargetName::Claude).expect("decided");
        if expected != Source::Layer {
            prop_assert_eq!(found.source, expected);
        }
        // Codex is not named by the allowed entry: skipped, never failed by step 4.
        let codex = result.target(TargetName::Codex).expect("decided");
        if codex.source == Source::Default {
            prop_assert!(matches!(codex.verdict, Verdict::Applied(Applied::Default)));
        }
    }
}
