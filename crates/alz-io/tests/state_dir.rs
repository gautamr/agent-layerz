//! The writer of `allowed.yaml` (spec §7.13 "Both files", decision-20; TASK-55.7.12). Every test
//! runs with a temporary `HOME` and `XDG_STATE_HOME`; none touches the real state folder, and none
//! changes the environment: the environment is a value.

#[path = "support/account_machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::mpsc;
    use std::time::Duration;

    use alz_core::account::{
        AccountName, AllowEntry, AllowFileProblem, AllowState, AllowedSource, NonEmptyTargetMap,
    };
    use alz_core::env::EnvVars;
    use alz_core::path::AbsolutePath;
    use alz_core::target::TargetName;
    use alz_io::{
        AccountFiles, AllowChange, EnvSnapshot, Home, IgnoreReason, RepositoryPaths, Stage,
        StateDir, StateError, Updated,
    };
    use proptest::prelude::*;

    use super::machine::Machine;

    const HEADER: &str =
        "# written by alz account allow; deleting the file withdraws every approval\n";
    const STATE: &str = "home/.local/state/agent-layerz";
    const FILE: &str = "home/.local/state/agent-layerz/allowed.yaml";

    fn env(machine: &Machine, more: &[(&str, &str)]) -> EnvSnapshot {
        let home = machine.path("home");
        EnvSnapshot::from_vars(
            [("HOME", home.to_str().expect("text"))]
                .into_iter()
                .chain(more.iter().copied())
                .map(|(name, value)| (OsString::from(name), OsString::from(value)))
                .collect::<EnvVars>(),
        )
    }

    /// The writer for the repository at `repo` of `machine`.
    fn writer_for(machine: &Machine, repo: &str, more: &[(&str, &str)]) -> StateDir {
        let env = env(machine, more);
        let repository = RepositoryPaths::new(machine.abs(repo), machine.abs(repo));
        StateDir::new(AccountFiles::new(
            Home::locate(&env),
            &env,
            Some(repository),
        ))
        .with_lock_wait(Duration::from_millis(300))
    }

    fn writer(machine: &Machine) -> StateDir {
        writer_for(machine, "repo", &[])
    }

    fn entry(machine: &Machine, repo: &str, claude: &str, source: &str) -> AllowEntry {
        entry_at(machine.abs(repo), claude, source)
    }

    fn entry_at(root: AbsolutePath, claude: &str, source: &str) -> AllowEntry {
        AllowEntry::new(
            root,
            NonEmptyTargetMap::from_entries([(
                TargetName::Claude,
                AccountName::try_from(claude).expect("a name"),
            )])
            .expect("a target"),
            AllowedSource::try_from(source).expect("a source"),
        )
    }

    fn set(machine: &Machine, repo: &str, claude: &str) -> AllowChange {
        AllowChange::Set(entry(machine, repo, claude, "/hub/layer"))
    }

    /// Every path below `root` with its kind, mode and content or link target, links not followed.
    fn snapshot(root: &Path) -> BTreeMap<PathBuf, (String, u32, Vec<u8>)> {
        fn walk(path: &Path, into: &mut BTreeMap<PathBuf, (String, u32, Vec<u8>)>) {
            let status = fs::symlink_metadata(path).expect("looked at");
            let kind = status.file_type();
            let (name, content) = if kind.is_symlink() {
                (
                    "link",
                    fs::read_link(path)
                        .expect("read")
                        .to_string_lossy()
                        .into_owned()
                        .into_bytes(),
                )
            } else if kind.is_dir() {
                ("dir", Vec::new())
            } else if kind.is_file() {
                ("file", fs::read(path).expect("read"))
            } else {
                ("other", Vec::new())
            };
            into.insert(
                path.to_path_buf(),
                (name.to_owned(), status.mode(), content),
            );
            if kind.is_dir() {
                for child in fs::read_dir(path).expect("listed") {
                    walk(&child.expect("entry").path(), into);
                }
            }
        }
        let mut found = BTreeMap::new();
        walk(root, &mut found);
        found
    }

    fn ran(result: Result<Updated, StateError>) -> Updated {
        result.unwrap_or_else(|error| panic!("{error:?}"))
    }

    fn text(machine: &Machine, relative: &str) -> String {
        fs::read_to_string(machine.path(relative)).expect("read")
    }

    fn mode_of(machine: &Machine, relative: &str) -> u32 {
        fs::metadata(machine.path(relative))
            .expect("looked at")
            .permissions()
            .mode()
            & 0o777
    }

    fn chmod(machine: &Machine, relative: &str, mode: u32) {
        fs::set_permissions(machine.path(relative), fs::Permissions::from_mode(mode))
            .expect("mode set");
    }

    fn put_state(machine: &Machine, content: &str) {
        machine.make_dirs(STATE);
        machine.write(FILE, content);
    }

    fn names(machine: &Machine, relative: &str) -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(machine.path(relative))
            .expect("listed")
            .map(|item| {
                item.expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        found.sort();
        found
    }

    // ---- what is written ----

    #[test]
    fn the_written_bytes_equal_the_samples_and_the_core_emitter() {
        let machine = Machine::new();
        let first = entry(&machine, "repo", "client-a", "/hub/layer");
        writer(&machine)
            .update(&AllowChange::Set(first.clone()))
            .expect("written");
        // The sample for one entry with two targets is checked in
        // the core's unit test; here the same text is recorded for the file itself.
        let sample = format!(
            "{HEADER}- root: {}\n  account:\n    claude: client-a\n  source: /hub/layer\n",
            machine.path("repo").display()
        );
        assert_eq!(text(&machine, FILE), sample);
        let other = entry(
            &machine,
            "other",
            "b-account",
            "https://example.test/hub.git",
        );
        writer_for(&machine, "other", &[])
            .update(&AllowChange::Set(other.clone()))
            .expect("written");
        assert_eq!(
            text(&machine, FILE),
            AllowState::from_entries(vec![first, other]).to_yaml()
        );
    }

    #[test]
    fn the_file_is_mode_0600_and_the_missing_folders_0700_while_existing_ones_keep_their_mode() {
        let machine = Machine::new();
        machine.make_dirs("home/.local");
        chmod(&machine, "home/.local", 0o750);
        writer(&machine)
            .update(&set(&machine, "repo", "a"))
            .expect("written");
        assert_eq!(mode_of(&machine, "home/.local"), 0o750);
        assert_eq!(mode_of(&machine, "home/.local/state"), 0o700);
        assert_eq!(mode_of(&machine, STATE), 0o700);
        assert_eq!(mode_of(&machine, FILE), 0o600);
        assert_eq!(
            names(&machine, STATE),
            ["allowed.yaml"],
            "no lock or temporary file is left behind"
        );
    }

    #[test]
    fn an_entry_is_replaced_by_root_others_stay_sorted_and_unresolved_roots_are_kept() {
        let machine = Machine::new();
        let stale = entry_at(
            AbsolutePath::try_from("/zzz/does/not/exist").expect("absolute"),
            "z",
            "/z",
        );
        let kept = entry(&machine, "other", "b", "/b");
        put_state(
            &machine,
            &AllowState::from_entries(vec![stale.clone(), kept.clone()]).to_yaml(),
        );
        chmod(&machine, FILE, 0o600);
        writer(&machine)
            .update(&set(&machine, "repo", "one"))
            .expect("written");
        let replacement = entry(&machine, "repo", "two", "/hub/layer");
        assert_eq!(
            ran(writer(&machine).update(&AllowChange::Set(replacement.clone()))),
            Updated::Written
        );
        assert_eq!(
            text(&machine, FILE),
            AllowState::from_entries(vec![stale, kept, replacement]).to_yaml()
        );
    }

    #[test]
    fn an_entry_already_recorded_changes_nothing_and_the_last_remove_leaves_an_empty_list() {
        let machine = Machine::new();
        let change = set(&machine, "repo", "a");
        assert_eq!(ran(writer(&machine).update(&change)), Updated::Written);
        let before = snapshot(&machine.path("home"));
        assert_eq!(ran(writer(&machine).update(&change)), Updated::Unchanged);
        assert_eq!(snapshot(&machine.path("home")), before);
        let remove = AllowChange::Remove(machine.abs("repo"));
        assert_eq!(ran(writer(&machine).update(&remove)), Updated::Written);
        assert_eq!(text(&machine, FILE), format!("{HEADER}[]\n"));
        assert_eq!(ran(writer(&machine).update(&remove)), Updated::Unchanged);
    }

    #[test]
    fn a_remove_with_no_file_creates_nothing_and_a_check_creates_no_folder() {
        let machine = Machine::new();
        let before = snapshot(&machine.path("home"));
        assert_eq!(
            ran(writer(&machine).update(&AllowChange::Remove(machine.abs("repo")))),
            Updated::Unchanged
        );
        writer(&machine).check().expect("can be written");
        assert_eq!(snapshot(&machine.path("home")), before);
    }

    #[test]
    fn a_result_over_1_mib_is_an_error_and_changes_nothing() {
        let machine = Machine::new();
        let before = snapshot(&machine.path("home"));
        let huge = entry(&machine, "repo", "a", &"s".repeat(1_100_000));
        assert!(matches!(
            writer(&machine).update(&AllowChange::Set(huge)),
            Err(StateError::TooLarge)
        ));
        assert_eq!(snapshot(&machine.path("home")), before);
    }

    // ---- refusals leave everything as it was ----

    /// Runs `change` after `arrange`, and asserts that it is refused and that `watched` is the same
    /// before and after.
    fn assert_refused(
        machine: &Machine,
        writer: &StateDir,
        change: &AllowChange,
        watched: &str,
    ) -> StateError {
        let before = snapshot(&machine.path(watched));
        let outcome = writer.update(change);
        assert_eq!(snapshot(&machine.path(watched)), before, "{outcome:?}");
        outcome.expect_err("a refusal")
    }

    #[test]
    fn a_state_folder_inside_the_repository_is_refused_and_nothing_is_written_there() {
        let machine = Machine::new();
        let inside = machine.path("repo/state");
        let writer = writer_for(
            &machine,
            "repo",
            &[("XDG_STATE_HOME", inside.to_str().unwrap())],
        );
        let error = assert_refused(&machine, &writer, &set(&machine, "repo", "a"), "repo");
        assert!(
            matches!(error, StateError::Place(IgnoreReason::InsideRepository)),
            "{error:?}"
        );
        assert!(!machine.path("repo/state").exists());
    }

    #[test]
    fn a_state_folder_in_an_account_folder_is_refused_before_any_folder_is_made() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude");
        let inside = machine.path("home/.claude/state");
        let writer = writer_for(
            &machine,
            "repo",
            &[("XDG_STATE_HOME", inside.to_str().unwrap())],
        );
        let error = assert_refused(&machine, &writer, &set(&machine, "repo", "a"), "home");
        assert!(
            matches!(error, StateError::Place(IgnoreReason::NamesAccountFolder)),
            "{error:?}"
        );
    }

    #[test]
    fn a_state_folder_in_a_git_work_tree_is_refused_missing_or_not() {
        for existing in [false, true] {
            let machine = Machine::new();
            machine.make_dirs("other/work/.git");
            if existing {
                machine.make_dirs("other/work/s/agent-layerz");
                machine.write("other/work/s/agent-layerz/allowed.yaml", "[]\n");
                chmod(&machine, "other/work/s/agent-layerz/allowed.yaml", 0o600);
            }
            let inside = machine.path("other/work/s");
            let writer = writer_for(
                &machine,
                "repo",
                &[("XDG_STATE_HOME", inside.to_str().unwrap())],
            );
            let error = assert_refused(&machine, &writer, &set(&machine, "repo", "a"), "other");
            assert!(
                matches!(error, StateError::Place(IgnoreReason::InsideWorkTree(_))),
                "{existing}: {error:?}"
            );
        }
    }

    #[test]
    fn an_empty_or_relative_xdg_state_home_counts_as_unset_as_the_spec_says() {
        for given in ["", "relative/state"] {
            let machine = Machine::new();
            let writer = writer_for(&machine, "repo", &[("XDG_STATE_HOME", given)]);
            assert_eq!(
                ran(writer.update(&set(&machine, "repo", "a"))),
                Updated::Written
            );
            assert!(machine.path(FILE).is_file(), "{given:?}");
        }
    }

    #[test]
    fn a_moved_state_folder_is_refused_while_mise_or_direnv_is_active() {
        let machine = Machine::new();
        let moved = machine.path("other/state");
        let writer = writer_for(
            &machine,
            "repo",
            &[
                ("XDG_STATE_HOME", moved.to_str().unwrap()),
                ("__MISE_DIFF", "x"),
            ],
        );
        let error = assert_refused(&machine, &writer, &set(&machine, "repo", "a"), "other");
        assert!(
            matches!(
                error,
                StateError::Place(IgnoreReason::XdgMoved("XDG_STATE_HOME"))
            ),
            "{error:?}"
        );
    }

    #[test]
    fn a_home_that_is_not_absolute_and_a_missing_user_id_are_refused() {
        let machine = Machine::new();
        let env = EnvSnapshot::from_vars(
            [(OsString::from("HOME"), OsString::from("relative"))]
                .into_iter()
                .collect::<EnvVars>(),
        );
        let files = AccountFiles::new(Home::locate(&env), &env, None);
        assert!(matches!(
            StateDir::new(files).update(&set(&machine, "repo", "a")),
            Err(StateError::Place(IgnoreReason::HomeNotAbsolute))
        ));
        let env = env_of(&machine);
        let files = AccountFiles::new(Home::locate(&env), &env, None).with_owner(None);
        assert!(matches!(
            StateDir::new(files).update(&set(&machine, "repo", "a")),
            Err(StateError::Place(IgnoreReason::NoUserId))
        ));
    }

    type Arrange = Box<dyn Fn(&Machine) -> StateDir>;

    fn env_of(machine: &Machine) -> EnvSnapshot {
        env(machine, &[])
    }

    #[test]
    fn a_file_or_folder_with_the_wrong_owner_or_mode_is_refused() {
        let wrong_owner = |machine: &Machine| {
            let env = env_of(machine);
            let owner = rustix::process::geteuid().as_raw().wrapping_add(1);
            StateDir::new(AccountFiles::new(Home::locate(&env), &env, None).with_owner(Some(owner)))
        };
        let cases: [(&str, Arrange); 3] = [
            (
                "file group-writable",
                Box::new(|m| {
                    chmod(m, FILE, 0o664);
                    writer(m)
                }),
            ),
            (
                "folder group-writable",
                Box::new(|m| {
                    chmod(m, STATE, 0o775);
                    writer(m)
                }),
            ),
            ("owned by another user", Box::new(wrong_owner)),
        ];
        for (name, arrange) in cases {
            let machine = Machine::new();
            put_state(&machine, &format!("{HEADER}[]\n"));
            chmod(&machine, FILE, 0o600);
            let writer = arrange(&machine);
            let error = assert_refused(&machine, &writer, &set(&machine, "repo", "a"), "home");
            assert!(matches!(error, StateError::Place(_)), "{name}: {error:?}");
        }
    }

    #[test]
    fn a_dangling_link_and_anything_that_is_not_a_regular_file_are_refused() {
        let machine = Machine::new();
        machine.make_dirs(STATE);
        machine.link(FILE, "nowhere.yaml");
        let error = assert_refused(
            &machine,
            &writer(&machine),
            &set(&machine, "repo", "a"),
            "home",
        );
        assert!(matches!(error, StateError::DanglingLink), "{error:?}");
        fs::remove_file(machine.path(FILE)).expect("removed");
        fs::create_dir(machine.path(FILE)).expect("made");
        let error = assert_refused(
            &machine,
            &writer(&machine),
            &set(&machine, "repo", "a"),
            "home",
        );
        assert!(matches!(error, StateError::Place(_)), "{error:?}");
    }

    #[test]
    fn a_file_that_is_not_a_list_or_holds_an_ignored_entry_is_refused_by_position() {
        let machine = Machine::new();
        let two_bad = format!(
            "- root: relative\n  account:\n    claude: a\n  source: /s\n- root: {}/../other\n  account:\n    claude: a\n  source: /s\n",
            machine.path("other").display()
        );
        let cases = [
            ("a: 1\n".to_owned(), AllowFileProblem::NotAList),
            (
                two_bad,
                AllowFileProblem::Entries(vec![
                    1.try_into().expect("non-zero"),
                    2.try_into().expect("non-zero"),
                ]),
            ),
        ];
        for (content, problem) in cases {
            put_state(&machine, &content);
            chmod(&machine, FILE, 0o600);
            let error = assert_refused(
                &machine,
                &writer(&machine),
                &set(&machine, "repo", "a"),
                "home",
            );
            match (&error, &problem) {
                (StateError::State(found), expected) => {
                    assert_eq!(found, expected, "{content}");
                    // The refusal and the account note word it alike.
                    assert_eq!(error.to_string(), found.state().to_string());
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_file_that_is_not_valid_yaml_is_an_ignored_file_and_refused_with_that_reason() {
        let machine = Machine::new();
        put_state(&machine, "[\n");
        chmod(&machine, FILE, 0o600);
        let error = assert_refused(
            &machine,
            &writer(&machine),
            &set(&machine, "repo", "a"),
            "home",
        );
        assert!(
            matches!(error, StateError::Place(IgnoreReason::NotValidYaml)),
            "{error:?}"
        );
    }

    // ---- links ----

    #[test]
    fn a_link_is_followed_to_a_file_alz_wrote_and_kept_a_link() {
        let machine = Machine::new();
        machine.make_dirs("other/store");
        machine.write("other/store/target.yaml", &format!("{HEADER}[]\n"));
        chmod(&machine, "other/store/target.yaml", 0o600);
        machine.make_dirs(STATE);
        machine.link(
            FILE,
            machine.path("other/store/target.yaml").to_str().unwrap(),
        );
        assert_eq!(
            ran(writer(&machine).update(&set(&machine, "repo", "a"))),
            Updated::Written
        );
        assert!(
            fs::symlink_metadata(machine.path(FILE))
                .expect("looked at")
                .file_type()
                .is_symlink()
        );
        assert!(text(&machine, "other/store/target.yaml").contains("claude: a"));
        assert_eq!(
            names(&machine, "other/store"),
            ["target.yaml"],
            "the lock and the temporary file are named for the real file and removed"
        );
    }

    #[test]
    fn a_link_to_somebody_elses_file_is_refused_and_the_file_is_untouched() {
        let machine = Machine::new();
        machine.make_dirs("other/store");
        machine.write("other/store/target.yaml", "- not: ours\n");
        chmod(&machine, "other/store/target.yaml", 0o600);
        machine.make_dirs(STATE);
        machine.link(
            FILE,
            machine.path("other/store/target.yaml").to_str().unwrap(),
        );
        let error = assert_refused(
            &machine,
            &writer(&machine),
            &set(&machine, "repo", "a"),
            "other",
        );
        assert!(matches!(error, StateError::LinkedElsewhere), "{error:?}");
        assert_eq!(
            writer(&machine).check().expect_err("refused").to_string(),
            error.to_string()
        );
    }

    #[test]
    fn check_state_looks_at_the_files_state_alone_and_check_also_at_the_link() {
        let machine = Machine::new();
        machine.make_dirs("other/store");
        machine.write(
            "other/store/target.yaml",
            "- root: /x\n  account:\n    claude: a\n  source: s\n",
        );
        chmod(&machine, "other/store/target.yaml", 0o600);
        machine.make_dirs(STATE);
        machine.link(
            FILE,
            machine.path("other/store/target.yaml").to_str().unwrap(),
        );
        let linked = writer(&machine);
        assert!(
            linked.check_state().is_ok(),
            "a valid list in a linked file"
        );
        assert!(matches!(
            linked.check().expect_err("refused"),
            StateError::LinkedElsewhere
        ));

        let top = Machine::new();
        top.make_dirs(STATE);
        top.write(FILE, "a: 1\n");
        chmod(&top, FILE, 0o600);
        assert!(matches!(
            writer(&top).check_state().expect_err("refused"),
            StateError::State(AllowFileProblem::NotAList)
        ));
    }

    #[test]
    fn a_target_swapped_before_the_rename_is_refused_and_the_temporary_file_removed() {
        let machine = Machine::new();
        machine.make_dirs("other/store");
        machine.write("other/store/target.yaml", &format!("{HEADER}[]\n"));
        chmod(&machine, "other/store/target.yaml", 0o600);
        machine.make_dirs(STATE);
        machine.link(
            FILE,
            machine.path("other/store/target.yaml").to_str().unwrap(),
        );
        let swap = |stage: Stage<'_>| {
            if matches!(stage, Stage::TempWritten(_)) {
                let elsewhere = machine.path("other/store/swapped.yaml");
                fs::write(&elsewhere, "- somebody: else\n").expect("written");
                fs::rename(&elsewhere, machine.path("other/store/target.yaml")).expect("renamed");
            }
        };
        let outcome = writer(&machine).update_observed(&set(&machine, "repo", "a"), &swap);
        assert!(
            matches!(outcome, Err(StateError::TargetChanged)),
            "{outcome:?}"
        );
        assert_eq!(
            text(&machine, "other/store/target.yaml"),
            "- somebody: else\n"
        );
        assert_eq!(names(&machine, "other/store"), ["target.yaml"]);
    }

    // ---- the lock ----

    #[test]
    fn a_lock_file_that_exists_refuses_by_name_after_the_wait_and_is_never_removed_or_followed() {
        let machine = Machine::new();
        put_state(&machine, &format!("{HEADER}[]\n"));
        chmod(&machine, FILE, 0o600);
        machine.write(&format!("{STATE}/allowed.yaml.lock"), "99999\n");
        let before = snapshot(&machine.path("home"));
        let outcome = writer(&machine).update(&set(&machine, "repo", "a"));
        match outcome {
            Err(StateError::LockHeld(shown)) => {
                assert!(shown.to_string().ends_with("allowed.yaml.lock"), "{shown}");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(snapshot(&machine.path("home")), before);
        // A lock that is a link is never followed: nothing is created at its target.
        fs::remove_file(machine.path(&format!("{STATE}/allowed.yaml.lock"))).expect("removed");
        machine.link(&format!("{STATE}/allowed.yaml.lock"), "planted");
        assert!(matches!(
            writer(&machine).update(&set(&machine, "repo", "a")),
            Err(StateError::LockHeld(_))
        ));
        assert!(!machine.path(&format!("{STATE}/planted")).exists());
    }

    #[test]
    fn a_second_writer_waits_for_the_first_and_both_entries_are_kept() {
        let machine = Machine::new();
        machine.make_dirs("other/second");
        let first = writer_for(&machine, "repo", &[]).with_lock_wait(Duration::from_secs(10));
        let second =
            writer_for(&machine, "other/second", &[]).with_lock_wait(Duration::from_secs(10));
        let (held_tx, held_rx) = mpsc::channel::<()>();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let go_rx = Mutex::new(go_rx);
        let held_tx = Mutex::new(held_tx);
        let go_tx = Mutex::new(go_tx);
        std::thread::scope(|scope| {
            let a = scope.spawn(|| {
                first.update_observed(&set(&machine, "repo", "a"), &|stage| {
                    if matches!(stage, Stage::LockHeld) {
                        held_tx.lock().expect("lock").send(()).expect("sent");
                        go_rx.lock().expect("lock").recv().expect("go");
                    }
                })
            });
            held_rx.recv().expect("the first holds the lock");
            let b = scope.spawn(|| {
                second.update_observed(&set(&machine, "other/second", "b"), &|stage| {
                    if matches!(stage, Stage::LockBusy) {
                        go_tx.lock().expect("lock").send(()).expect("sent");
                    }
                })
            });
            assert_eq!(ran(a.join().expect("joined")), Updated::Written);
            assert_eq!(ran(b.join().expect("joined")), Updated::Written);
        });
        let written = text(&machine, FILE);
        assert!(
            written.contains("claude: a") && written.contains("claude: b"),
            "{written}"
        );
        assert_eq!(names(&machine, STATE), ["allowed.yaml"]);
    }

    #[test]
    fn twelve_writers_at_once_keep_every_entry_and_each_writes_or_names_the_lock_file() {
        let machine = Machine::new();
        for index in 0..12 {
            machine.make_dirs(&format!("other/r{index}"));
        }
        let results: Vec<Result<Updated, StateError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..12)
                .map(|index| {
                    let machine = &machine;
                    scope.spawn(move || {
                        let repo = format!("other/r{index}");
                        writer_for(machine, &repo, &[])
                            .with_lock_wait(Duration::from_secs(10))
                            .update(&set(machine, &repo, &format!("acct{index}")))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("joined"))
                .collect()
        });
        let written = text(&machine, FILE);
        for (index, result) in results.iter().enumerate() {
            match result {
                Ok(Updated::Written) => {
                    assert!(
                        written.contains(&format!("claude: acct{index}\n")),
                        "{index}"
                    );
                }
                Err(StateError::LockHeld(shown)) => {
                    assert!(shown.to_string().contains("allowed.yaml.lock"));
                }
                other => panic!("writer {index}: {other:?}"),
            }
        }
        assert!(
            results
                .iter()
                .any(|result| matches!(result, Ok(Updated::Written)))
        );
        assert_eq!(names(&machine, STATE), ["allowed.yaml"]);
    }

    // ---- a crash ----

    const CRASH_HOME: &str = "ALZ_STATE_DIR_TEST_CRASH_HOME";

    #[expect(
        clippy::disallowed_methods,
        reason = "the re-run of this test binary receives its folders through its environment"
    )]
    fn crash_home() -> Option<OsString> {
        std::env::var_os(CRASH_HOME)
    }

    /// Not a test of its own: the child of the crash test below. It writes, and the process aborts
    /// when the temporary file is written and the rename has not happened.
    #[test]
    fn the_child_that_crashes_before_the_rename() {
        let Some(home) = crash_home() else { return };
        let home = PathBuf::from(home);
        let root = home.parent().expect("a parent").to_path_buf();
        let env = EnvSnapshot::from_vars(
            [(OsString::from("HOME"), home.clone().into_os_string())]
                .into_iter()
                .collect::<EnvVars>(),
        );
        let repo = AbsolutePath::try_from(root.join("repo").to_str().expect("text")).expect("abs");
        let files = AccountFiles::new(
            Home::locate(&env),
            &env,
            Some(RepositoryPaths::new(repo.clone(), repo.clone())),
        );
        let change = AllowChange::Set(entry_at(repo, "new", "/hub/layer"));
        drop(StateDir::new(files).update_observed(&change, &|stage| {
            if matches!(stage, Stage::TempWritten(_)) {
                std::process::abort();
            }
        }));
    }

    #[test]
    fn a_crash_between_the_temporary_file_and_the_rename_leaves_the_old_file_whole() {
        let machine = Machine::new();
        let old = AllowState::from_entries(vec![entry(&machine, "other", "old", "/old")]).to_yaml();
        put_state(&machine, &old);
        chmod(&machine, FILE, 0o600);
        let status = std::process::Command::new(
            alz_test_support::test_executable().expect("the test binary"),
        )
        .args([
            "--exact",
            "tests::the_child_that_crashes_before_the_rename",
            "--nocapture",
        ])
        .env(CRASH_HOME, machine.path("home"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("started");
        assert!(!status.success(), "the child aborted");
        assert_eq!(text(&machine, FILE), old, "the old file is whole");
        let left = names(&machine, STATE);
        let temp: Vec<&String> = left.iter().filter(|name| name.contains(".tmp")).collect();
        assert_eq!(temp.len(), 1, "{left:?}");
        assert!(temp[0].starts_with(".allowed.yaml."), "{left:?}");
        assert_eq!(mode_of(&machine, &format!("{STATE}/{}", temp[0])), 0o600);
        // alz never removes a lock it did not create: the next run names it.
        assert!(left.contains(&"allowed.yaml.lock".to_owned()), "{left:?}");
        assert!(matches!(
            writer(&machine).update(&set(&machine, "repo", "a")),
            Err(StateError::LockHeld(_))
        ));
        assert_eq!(text(&machine, FILE), old);
    }

    // ---- generated places ----

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]

        /// Whatever the state folder below the repository, an account folder or a git work tree,
        /// the write is refused and nothing is made in any of them.
        #[test]
        fn nothing_is_ever_written_inside_the_repository_an_account_folder_or_a_work_tree(
            base in 0usize..4,
            below in proptest::collection::vec("[a-z]{1,6}", 1..4),
        ) {
            let machine = Machine::new();
            machine.make_dirs("home/.claude");
            machine.make_dirs("home/.codex-work");
            machine.make_dirs("other/tree/.git");
            let base = ["repo", "home/.claude", "home/.codex-work", "other/tree"][base];
            let given = machine.path(base).join(below.join("/"));
            let writer = writer_for(
                &machine,
                "repo",
                &[("XDG_STATE_HOME", given.to_str().expect("text"))],
            );
            let before = snapshot(&machine.path("."));
            let outcome = writer.update(&set(&machine, "repo", "a"));
            prop_assert!(matches!(outcome, Err(StateError::Place(_))), "{:?}", outcome);
            prop_assert_eq!(snapshot(&machine.path(".")), before);
        }
    }
}
