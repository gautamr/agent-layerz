//! The home folder and the two account files (spec §7.13 "Environment", "User config", "Allow
//! state", "Both files"; TASK-72, TASK-55.7.6), each trust check with a temporary `HOME`. No test
//! reads the real home folder, and none changes the environment: the environment is a value.

#[path = "support/account_machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use alz_core::env::EnvVars;
    use alz_io::{
        AccountFiles, EnvSnapshot, FileStatus, Home, IgnoreReason, Loaded, RepositoryPaths,
    };
    use proptest::prelude::*;

    use super::machine::Machine;

    /// The two files, each with a place below `home`, the variable that moves it, and content that
    /// is valid for it.
    #[derive(Debug, Clone, Copy)]
    enum Kind {
        Config,
        State,
    }

    const KINDS: [Kind; 2] = [Kind::Config, Kind::State];

    impl Kind {
        const fn name(self) -> &'static str {
            match self {
                Self::Config => "accounts.yaml",
                Self::State => "allowed.yaml",
            }
        }

        const fn variable(self) -> &'static str {
            match self {
                Self::Config => "XDG_CONFIG_HOME",
                Self::State => "XDG_STATE_HOME",
            }
        }

        /// The folder of the file, below `home`.
        const fn folder(self) -> &'static str {
            match self {
                Self::Config => "home/.config/agent-layerz",
                Self::State => "home/.local/state/agent-layerz",
            }
        }

        fn file(self) -> String {
            format!("{}/{}", self.folder(), self.name())
        }

        const fn valid(self) -> &'static str {
            match self {
                Self::Config => "default_name: personal\n",
                Self::State => "[]\n",
            }
        }
    }

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

    fn files(machine: &Machine, more: &[(&str, &str)]) -> AccountFiles {
        let env = env(machine, more);
        AccountFiles::new(Home::locate(&env), &env, None)
    }

    /// Writes `text` as the file of `kind`, in folders of mode 0755 as a user makes them.
    fn put(machine: &Machine, kind: Kind, text: &str) {
        machine.make_dirs(kind.folder());
        machine.write(&kind.file(), text);
    }

    fn mode(machine: &Machine, relative: &str, mode: u32) {
        fs::set_permissions(machine.path(relative), fs::Permissions::from_mode(mode))
            .expect("mode set");
    }

    fn owner() -> u32 {
        rustix::process::geteuid().as_raw()
    }

    /// Status and warnings of one kind, as text.
    fn look(files: &AccountFiles, kind: Kind) -> (FileStatus, Vec<String>) {
        match kind {
            Kind::Config => summary(&files.user_config()),
            Kind::State => summary(&files.allow_state()),
        }
    }

    fn summary<T>(loaded: &Loaded<T>) -> (FileStatus, Vec<String>) {
        (
            loaded.status().clone(),
            loaded.warnings().iter().map(ToString::to_string).collect(),
        )
    }

    fn ignored(status: &FileStatus) -> &IgnoreReason {
        match status {
            FileStatus::Ignored(reason) => reason,
            other @ (FileStatus::Read | FileStatus::NotFound) => {
                panic!("expected an ignored file, got {other:?}")
            }
        }
    }

    fn assert_ignored_with(kind: Kind, files: &AccountFiles, reason: &IgnoreReason, text: &str) {
        let (status, warnings) = look(files, kind);
        assert_eq!(ignored(&status), reason, "{kind:?}");
        assert_eq!(warnings.len(), 1, "{kind:?}: {warnings:?}");
        assert!(
            warnings.iter().all(|line| line.contains(text)),
            "{kind:?}: {warnings:?} should contain {text:?}"
        );
        assert!(
            warnings
                .iter()
                .all(|line| line.contains(&format!("{} (", kind.name()))),
            "{kind:?}: {warnings:?}"
        );
    }

    // ----- Home (TASK-72) -----

    #[test]
    fn with_home_unset_and_no_user_database_entry_both_files_are_ignored_with_one_warning() {
        let env = EnvSnapshot::from_vars(EnvVars::default());
        let home = Home::locate_with(&env, || None);
        assert_eq!(home, Home::NotAbsolute);
        let files = AccountFiles::new(home, &env, None);
        let warning = files.home_warning().expect("one warning").to_string();
        assert_eq!(
            warning,
            "warning [account-home] HOME is not an absolute path; accounts.yaml and allowed.yaml \
             are ignored and no account folder is set"
        );
        for kind in KINDS {
            let (status, warnings) = look(&files, kind);
            assert_eq!(status, FileStatus::Ignored(IgnoreReason::HomeNotAbsolute));
            assert!(
                warnings.is_empty(),
                "{kind:?}: the home warning is not repeated"
            );
        }
        assert_eq!(files.user_config().path(), None);
        assert_eq!(files.user_config().value().folders().len(), 0);
        assert_eq!(files.allow_state().value().entries().len(), 0);
    }

    #[test]
    fn a_relative_or_empty_home_ignores_both_files_without_reading_the_user_database() {
        for given in ["", "relative/home"] {
            let env = EnvSnapshot::from_vars(
                [(OsString::from("HOME"), OsString::from(given))]
                    .into_iter()
                    .collect::<EnvVars>(),
            );
            let files = AccountFiles::new(Home::locate_with(&env, || None), &env, None);
            assert!(files.home_warning().is_some(), "{given:?}");
            for kind in KINDS {
                assert_eq!(
                    look(&files, kind).0,
                    FileStatus::Ignored(IgnoreReason::HomeNotAbsolute)
                );
            }
        }
    }

    #[test]
    fn an_unset_home_found_in_the_user_database_reads_the_files_there() {
        let machine = Machine::new();
        put(&machine, Kind::Config, Kind::Config.valid());
        let env = EnvSnapshot::from_vars(EnvVars::default());
        let found = machine.path("home");
        let home = Home::locate_with(&env, || Some(found));
        let files = AccountFiles::new(home, &env, None);
        assert_eq!(files.home_warning(), None);
        assert_eq!(look(&files, Kind::Config), (FileStatus::Read, vec![]));
    }

    #[test]
    fn a_home_that_is_a_link_is_used_as_given_and_its_files_are_read() {
        let machine = Machine::new();
        put(&machine, Kind::Config, "default_name: personal\n");
        machine.link("homelink", machine.path("home").to_str().expect("text"));
        let given = machine.path("homelink");
        let env = EnvSnapshot::from_vars(
            [(
                OsString::from("HOME"),
                OsString::from(given.to_str().expect("text")),
            )]
            .into_iter()
            .collect::<EnvVars>(),
        );
        let files = AccountFiles::new(Home::locate(&env), &env, None);
        let loaded = files.user_config();
        assert_eq!(loaded.status(), &FileStatus::Read);
        assert_eq!(
            loaded.path().map(ToString::to_string),
            Some(format!(
                "{}/.config/agent-layerz/accounts.yaml",
                given.display()
            ))
        );
        assert_eq!(
            loaded
                .value()
                .default_name()
                .map(ToString::to_string)
                .as_deref(),
            Some("personal")
        );
    }

    // ----- trusted and absent files -----

    #[test]
    fn a_file_that_passes_every_check_is_read() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            assert_eq!(
                look(&files(&machine, &[]), kind),
                (FileStatus::Read, vec![]),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_file_that_does_not_exist_is_not_found_and_is_not_a_problem() {
        let machine = Machine::new();
        for kind in KINDS {
            assert_eq!(
                look(&files(&machine, &[]), kind),
                (FileStatus::NotFound, vec![]),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_file_of_exactly_one_mebibyte_is_read_and_one_byte_more_is_ignored() {
        let machine = Machine::new();
        for kind in KINDS {
            let comment = |bytes: usize| format!("#{}\n", "x".repeat(bytes - 2));
            put(&machine, kind, &comment(1_048_576));
            assert_eq!(look(&files(&machine, &[]), kind).0, FileStatus::Read);
            put(&machine, kind, &comment(1_048_577));
            assert_ignored_with(
                kind,
                &files(&machine, &[]),
                &IgnoreReason::Unreadable,
                "it is not a regular file of at most 1 MiB that can be read",
            );
        }
    }

    // ----- each ignore reason of "Both files" -----

    #[test]
    fn a_platform_without_a_user_id_ignores_the_files() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            assert_ignored_with(
                kind,
                &files(&machine, &[]).with_owner(None),
                &IgnoreReason::NoUserId,
                "the owner of the file cannot be checked on this platform",
            );
        }
    }

    #[test]
    fn a_file_the_user_does_not_own_is_ignored() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            assert_ignored_with(
                kind,
                &files(&machine, &[]).with_owner(Some(owner().wrapping_add(1))),
                &IgnoreReason::NotOwned,
                "it is not owned by you",
            );
        }
    }

    #[test]
    fn a_file_that_group_or_others_can_write_is_ignored() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            for bits in [0o664, 0o646, 0o666] {
                mode(&machine, &kind.file(), bits);
                assert_ignored_with(
                    kind,
                    &files(&machine, &[]),
                    &IgnoreReason::GroupOrOthersCanWrite,
                    "group or others can write it",
                );
            }
        }
    }

    #[test]
    fn a_directory_that_group_or_others_can_write_is_ignored_for_the_folder_and_for_each_parent_below_home()
     {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            let parent = kind.folder().rsplit_once('/').expect("a parent").0;
            for folder in [kind.folder(), parent] {
                mode(&machine, folder, 0o775);
                let (status, warnings) = look(&files(&machine, &[]), kind);
                assert!(
                    matches!(ignored(&status), IgnoreReason::Directory { .. }),
                    "{kind:?} {folder}: {status:?}"
                );
                assert!(
                    warnings
                        .iter()
                        .any(|line| line.contains("can be written by group or others")
                            && line.contains(&format!("~/{}", folder.trim_start_matches("home/")))),
                    "{kind:?} {folder}: {warnings:?}"
                );
                mode(&machine, folder, 0o755);
            }
        }
    }

    #[test]
    fn a_file_in_a_folder_outside_home_is_checked_in_that_folder_only() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(&format!("other/{}/agent-layerz", kind.name()));
            machine.write(
                &format!("other/{}/agent-layerz/{}", kind.name(), kind.name()),
                kind.valid(),
            );
            let base = machine.path(&format!("other/{}", kind.name()));
            let files = files(&machine, &[(kind.variable(), base.to_str().expect("text"))]);
            assert_eq!(look(&files, kind), (FileStatus::Read, vec![]), "{kind:?}");
            mode(
                &machine,
                &format!("other/{}/agent-layerz", kind.name()),
                0o777,
            );
            assert!(matches!(
                ignored(&look(&files, kind).0),
                IgnoreReason::Directory { .. }
            ));
        }
    }

    #[test]
    fn a_link_is_followed_and_the_folder_of_its_target_is_checked_as_well() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(&format!("other/{}", kind.name()));
            let target = machine.path(&format!("other/{}/real.yaml", kind.name()));
            fs::write(&target, kind.valid()).expect("written");
            machine.make_dirs(kind.folder());
            machine.link(&kind.file(), target.to_str().expect("text"));
            assert_eq!(
                look(&files(&machine, &[]), kind),
                (FileStatus::Read, vec![]),
                "{kind:?}"
            );
            mode(&machine, &format!("other/{}", kind.name()), 0o777);
            let (status, warnings) = look(&files(&machine, &[]), kind);
            assert!(
                matches!(ignored(&status), IgnoreReason::Directory { .. }),
                "{kind:?}: {status:?} {warnings:?}"
            );
            fs::remove_file(machine.path(&kind.file())).expect("removed");
        }
    }

    #[test]
    fn a_file_whose_real_path_is_inside_an_account_folder_is_ignored() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs("home/.claude-work");
            machine.write("home/.claude-work/x.yaml", kind.valid());
            machine.make_dirs(kind.folder());
            machine.link(
                &kind.file(),
                machine
                    .path("home/.claude-work/x.yaml")
                    .to_str()
                    .expect("text"),
            );
            assert_ignored_with(
                kind,
                &files(&machine, &[]),
                &IgnoreReason::InsideAccountFolder,
                "it lies inside an account folder",
            );
            fs::remove_file(machine.path(&kind.file())).expect("removed");
        }
    }

    #[test]
    fn a_path_that_names_an_account_folder_as_written_is_ignored_before_the_file_system_is_asked() {
        let machine = Machine::new();
        for kind in KINDS {
            // Nothing exists there: a file system call would say "not found".
            let base = machine.path("home/.claude-work");
            assert_ignored_with(
                kind,
                &files(&machine, &[(kind.variable(), base.to_str().expect("text"))]),
                &IgnoreReason::NamesAccountFolder,
                "it lies inside an account folder",
            );
        }
    }

    #[test]
    fn a_file_inside_the_current_repository_is_ignored() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(&format!("repo/state/{}/agent-layerz", kind.name()));
            machine.write(
                &format!("repo/state/{}/agent-layerz/{}", kind.name(), kind.name()),
                kind.valid(),
            );
            let base = machine.path(&format!("repo/state/{}", kind.name()));
            let env = env(&machine, &[(kind.variable(), base.to_str().expect("text"))]);
            let repository = RepositoryPaths::new(machine.abs("repo"), machine.abs("repo"));
            let files = AccountFiles::new(Home::locate(&env), &env, Some(repository));
            assert_ignored_with(
                kind,
                &files,
                &IgnoreReason::InsideRepository,
                "it lies inside the current repository",
            );
        }
    }

    #[test]
    fn allowed_yaml_in_a_git_work_tree_is_ignored_and_accounts_yaml_only_warns() {
        let machine = Machine::new();
        machine.make_dirs("home/.git");
        put(&machine, Kind::State, "[]\n");
        let (status, warnings) = look(&files(&machine, &[]), Kind::State);
        assert!(
            matches!(ignored(&status), IgnoreReason::InsideWorkTree(_)),
            "{status:?}"
        );
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("it lies inside a git work tree (~/.git)"),
            "{warnings:?}"
        );
        put(&machine, Kind::Config, "default_name: personal\n");
        let (status, warnings) = look(&files(&machine, &[]), Kind::Config);
        assert_eq!(status, FileStatus::Read);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0]
                .contains("lies inside a git work tree (~/.git); keep it outside every repository"),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_work_tree_marker_that_is_a_file_counts_too() {
        let machine = Machine::new();
        machine.write("home/.git", "gitdir: elsewhere\n");
        put(&machine, Kind::State, "[]\n");
        assert!(matches!(
            ignored(&look(&files(&machine, &[]), Kind::State).0),
            IgnoreReason::InsideWorkTree(_)
        ));
    }

    #[test]
    fn a_variable_moved_while_mise_or_direnv_is_active_is_ignored_even_when_nothing_is_there() {
        let machine = Machine::new();
        for kind in KINDS {
            let moved = machine.path("other/moved");
            let moved = moved.to_str().expect("text");
            for manager in ["__MISE_DIFF", "DIRENV_DIFF"] {
                assert_ignored_with(
                    kind,
                    &files(&machine, &[(kind.variable(), moved), (manager, "")]),
                    &IgnoreReason::XdgMoved(kind.variable()),
                    &format!(
                        "{} points away from its default and mise or direnv is active here",
                        kind.variable()
                    ),
                );
            }
            assert_eq!(
                look(&files(&machine, &[(kind.variable(), moved)]), kind),
                (FileStatus::NotFound, vec![]),
                "without mise or direnv the variable is honoured: {kind:?}"
            );
            let default = match kind {
                Kind::Config => machine.path("home/.config"),
                Kind::State => machine.path("home/.local/state"),
            };
            assert_eq!(
                look(
                    &files(
                        &machine,
                        &[
                            (kind.variable(), default.to_str().expect("text")),
                            ("__MISE_DIFF", "")
                        ]
                    ),
                    kind
                ),
                (FileStatus::NotFound, vec![]),
                "a variable that equals its default is no move: {kind:?}"
            );
        }
    }

    #[test]
    fn an_xdg_variable_that_is_empty_counts_as_unset_and_a_relative_one_also_warns() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, kind.valid());
            assert_eq!(
                look(&files(&machine, &[(kind.variable(), "")]), kind),
                (FileStatus::Read, vec![]),
                "{kind:?}"
            );
            let (status, warnings) =
                look(&files(&machine, &[(kind.variable(), "relative/dir")]), kind);
            assert_eq!(status, FileStatus::Read, "{kind:?}");
            assert_eq!(
                warnings,
                vec![format!(
                    "warning [account-xdg-relative] {} is not an absolute path; {} is looked for in the default place",
                    kind.variable(),
                    kind.name()
                )]
            );
        }
    }

    /// Looks at a file on another thread, so a read that blocks fails the test and never hangs it.
    fn look_within_ten_seconds(files: AccountFiles, kind: Kind) -> Option<FileStatus> {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || sender.send(look(&files, kind).0).unwrap_or_default());
        receiver.recv_timeout(Duration::from_secs(10)).ok()
    }

    #[test]
    fn a_fifo_in_place_of_either_file_is_unreadable_and_nothing_waits() {
        for kind in KINDS {
            let machine = Machine::new();
            machine.make_dirs(kind.folder());
            let fifo = std::process::Command::new("mkfifo")
                .arg(machine.path(&kind.file()))
                .status()
                .expect("mkfifo runs");
            assert!(fifo.success());
            let status = look_within_ten_seconds(files(&machine, &[]), kind)
                .expect("reading a FIFO must not block");
            assert_eq!(
                status,
                FileStatus::Ignored(IgnoreReason::Unreadable),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_folder_in_place_of_a_file_is_unreadable() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(&kind.file());
            assert_ignored_with(
                kind,
                &files(&machine, &[]),
                &IgnoreReason::Unreadable,
                "it is not a regular file of at most 1 MiB that can be read",
            );
        }
    }

    #[test]
    fn a_path_through_a_loop_of_links_cannot_be_resolved() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(kind.folder());
            machine.link(&format!("{}/a", kind.folder()), "b");
            machine.link(&format!("{}/b", kind.folder()), "a");
            machine.link(&kind.file(), "a");
            assert_ignored_with(
                kind,
                &files(&machine, &[]),
                &IgnoreReason::PathUnresolvable,
                "its path cannot be resolved",
            );
        }
    }

    #[test]
    fn a_dangling_link_is_not_found() {
        let machine = Machine::new();
        for kind in KINDS {
            machine.make_dirs(kind.folder());
            machine.link(&kind.file(), "nowhere");
            assert_eq!(look(&files(&machine, &[]), kind).0, FileStatus::NotFound);
        }
    }

    #[test]
    fn a_file_that_is_not_valid_yaml_is_read_as_empty_with_one_warning_that_says_so() {
        let machine = Machine::new();
        for kind in KINDS {
            put(&machine, kind, "a: [unclosed\n");
            let (status, warnings) = look(&files(&machine, &[]), kind);
            assert_eq!(status, FileStatus::Read);
            assert_eq!(warnings.len(), 1, "{warnings:?}");
            assert!(warnings[0].contains("the file is ignored"), "{warnings:?}");
        }
    }

    #[test]
    fn a_warning_names_the_file_with_a_tilde_for_home_and_never_the_file_text() {
        let machine = Machine::new();
        put(&machine, Kind::Config, "default_name: SECRET-NAME\n");
        mode(&machine, &Kind::Config.file(), 0o666);
        let (_, warnings) = look(&files(&machine, &[]), Kind::Config);
        assert_eq!(
            warnings,
            vec![
                "warning [account-file-ignored] accounts.yaml (~/.config/agent-layerz/accounts.yaml) is \
                 ignored: group or others can write it"
                    .to_owned()
            ]
        );
    }

    // ----- the content, checked again once real -----

    fn config(machine: &Machine, text: &str) -> Loaded<alz_io::ResolvedUserConfig> {
        put(machine, Kind::Config, text);
        files(machine, &[]).user_config()
    }

    fn state(machine: &Machine, text: &str) -> Loaded<alz_io::ResolvedAllowState> {
        put(machine, Kind::State, text);
        files(machine, &[]).allow_state()
    }

    fn real_paths(loaded: &Loaded<alz_io::ResolvedUserConfig>) -> Vec<String> {
        loaded
            .value()
            .folders()
            .iter()
            .map(|folder| folder.real().to_string())
            .collect()
    }

    #[test]
    fn a_folders_key_is_resolved_to_its_real_path_and_one_that_does_not_exist_matches_nothing() {
        let machine = Machine::new();
        machine.make_dirs("home/work/a");
        machine.make_dirs("other/b");
        machine.link(
            "home/work/linked",
            machine.path("other/b").to_str().expect("text"),
        );
        let loaded = config(
            &machine,
            &format!(
                "folders:\n  ~/work/a: one\n  ~/work/linked: two\n  ~/work/missing: three\n  {}/other/b: two\n",
                machine.path("").display()
            ),
        );
        assert_eq!(
            real_paths(&loaded),
            [
                machine.abs("home/work/a").to_string(),
                machine.abs("other/b").to_string(),
                machine.abs("other/b").to_string()
            ]
        );
        assert!(loaded.warnings().iter().next().is_none());
    }

    #[test]
    fn a_folders_key_inside_an_account_folder_or_home_or_above_is_ignored_with_a_warning() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-work/sub");
        let loaded = config(
            &machine,
            &format!(
                "folders:\n  ~/.claude-work/sub: a\n  ~/: a\n  {}: a\n  /: a\n",
                machine.path("home").display()
            ),
        );
        assert_eq!(loaded.value().folders().len(), 0);
        assert_eq!(loaded.warnings().iter().count(), 4);
        assert!(loaded.warnings().iter().all(|warning| {
            warning.to_string().contains(
                "lies inside an account folder, or is $HOME or above it, so it is ignored",
            )
        }));
    }

    #[test]
    fn a_folders_key_that_reaches_an_account_folder_through_a_link_is_ignored() {
        let machine = Machine::new();
        machine.make_dirs("home/.codex-work");
        machine.make_dirs("home/work");
        machine.link(
            "home/work/via",
            machine.path("home/.codex-work").to_str().expect("text"),
        );
        let loaded = config(&machine, "folders:\n  ~/work/via: a\n");
        assert_eq!(loaded.value().folders().len(), 0);
        assert_eq!(loaded.warnings().iter().count(), 1);
    }

    #[test]
    fn two_keys_that_resolve_alike_with_different_accounts_are_both_ignored_and_equal_ones_are_kept()
     {
        let machine = Machine::new();
        machine.make_dirs("home/work/a");
        machine.link(
            "home/work/same",
            machine.path("home/work/a").to_str().expect("text"),
        );
        let loaded = config(&machine, "folders:\n  ~/work/a: one\n  ~/work/same: two\n");
        assert_eq!(loaded.value().folders().len(), 0);
        let lines: Vec<String> = loaded.warnings().iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            vec![
                "warning [account-config-duplicate] ~/.config/agent-layerz/accounts.yaml: \
                 two folders keys resolve to the same directory with different accounts; both are ignored"
                    .to_owned()
            ]
        );
        let same = config(&machine, "folders:\n  ~/work/a: one\n  ~/work/same: one\n");
        assert_eq!(same.value().folders().len(), 2);
        assert!(same.warnings().iter().next().is_none());
    }

    #[test]
    fn unusual_folders_keys_never_stop_the_read() {
        let machine = Machine::new();
        // Each row is one key as the YAML file writes it (a double-quoted scalar) and the number of
        // warnings it raises: a control character after the escapes ignores the whole file (§5.2).
        let rows: [(&str, usize); 10] = [
            (r#""~""#, 1),
            (r#""~/""#, 1),
            (r#""//""#, 1),
            (r#""/a\u0085b""#, 1),
            (r#""/a\u2028b""#, 1),
            (r#""/a\u001bb""#, 1),
            (r#""relative""#, 1),
            (r#""~user/x""#, 1),
            (r#""/a/../b""#, 0),
            (r#""~/ ""#, 0),
        ];
        for (key, warnings) in rows {
            let loaded = config(&machine, &format!("folders:\n  {key}: a\n"));
            assert_eq!(
                loaded.warnings().iter().count(),
                warnings,
                "{key}: {:?}",
                loaded
                    .warnings()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            );
            assert!(
                loaded
                    .warnings()
                    .iter()
                    .all(|warning| !warning.to_string().chars().any(char::is_control)),
                "{key}"
            );
        }
    }

    #[test]
    fn an_allowed_root_that_is_its_own_real_path_is_kept_and_one_that_does_not_exist_is_kept_apart()
    {
        let machine = Machine::new();
        machine.make_dirs("repo/api");
        let api = machine.abs("repo/api");
        let loaded = state(
            &machine,
            &format!(
                "- root: {api}\n  account: {{claude: one}}\n  source: s\n\
                 - root: {}/gone\n  account: {{claude: one}}\n  source: s\n",
                machine.path("repo").display()
            ),
        );
        let value = loaded.value();
        assert_eq!(value.entries().len(), 1);
        assert_eq!(value.entries()[0].root(), &api);
        assert_eq!(value.unresolved().len(), 1);
        assert_eq!(value.ignored().len(), 0);
        assert!(loaded.warnings().iter().next().is_none());
    }

    #[test]
    fn an_allowed_root_that_is_not_its_own_real_path_is_ignored_with_its_position_only() {
        let machine = Machine::new();
        machine.make_dirs("repo/api");
        machine.link(
            "repo/link",
            machine.path("repo/api").to_str().expect("text"),
        );
        let loaded = state(
            &machine,
            &format!(
                "- root: {}/repo/link\n  account: {{claude: one}}\n  source: s\n",
                machine.path("").display()
            ),
        );
        assert_eq!(loaded.value().entries().len(), 0);
        assert_eq!(
            loaded
                .value()
                .ignored()
                .iter()
                .map(|n| n.get())
                .collect::<Vec<_>>(),
            [1]
        );
        let lines: Vec<String> = loaded.warnings().iter().map(ToString::to_string).collect();
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].ends_with(
                "entry 1 is ignored (its root is not a real path (alz account allow records the real path))"
            ),
            "{lines:?}"
        );
    }

    #[test]
    fn an_allowed_root_inside_an_account_folder_or_home_or_above_is_ignored_and_one_that_only_holds_a_configured_folder_is_kept()
     {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-work/x");
        machine.make_dirs("repo/holder/inner");
        let configured = machine.path("repo/holder/inner");
        let configured = configured.to_str().expect("text");
        let home = machine.path("home");
        let env = env(&machine, &[("CLAUDE_CONFIG_DIR", configured)]);
        let text = format!(
            "- root: {home}/.claude-work/x\n  account: {{claude: a}}\n  source: s\n\
             - root: {home}\n  account: {{claude: a}}\n  source: s\n\
             - root: {holder}\n  account: {{claude: a}}\n  source: s\n",
            home = home.display(),
            holder = machine.path("repo/holder").display(),
        );
        put(&machine, Kind::State, &text);
        let loaded = AccountFiles::new(Home::locate(&env), &env, None).allow_state();
        assert_eq!(
            loaded
                .value()
                .entries()
                .iter()
                .map(|e| e.root().to_string())
                .collect::<Vec<_>>(),
            [machine.abs("repo/holder").to_string()]
        );
        assert_eq!(
            loaded
                .value()
                .ignored()
                .iter()
                .map(|n| n.get())
                .collect::<Vec<_>>(),
            [1, 2]
        );
    }

    #[test]
    fn two_entries_with_one_real_root_are_both_ignored_with_one_warning_naming_both_positions() {
        let machine = Machine::new();
        machine.make_dirs("repo/api");
        let api = machine.abs("repo/api");
        let entry = format!("- root: {api}\n  account: {{claude: a}}\n  source: s\n");
        let loaded = state(&machine, &format!("{entry}{entry}"));
        assert_eq!(loaded.value().entries().len(), 0);
        let lines: Vec<String> = loaded.warnings().iter().map(ToString::to_string).collect();
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].ends_with("entries 1, 2 are ignored (they have the same root)"),
            "{lines:?}"
        );
        assert_eq!(
            loaded
                .value()
                .ignored()
                .iter()
                .map(|n| n.get())
                .collect::<Vec<_>>(),
            [1, 2]
        );
    }

    // ----- properties -----

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]

        /// An ignored file contributes no entry, and nothing of its text reaches a warning.
        #[test]
        fn an_ignored_file_contributes_nothing_and_no_warning_holds_its_text(marker in "[A-Z]{8,12}") {
            let machine = Machine::new();
            put(&machine, Kind::Config, &format!(
                "default_name: {marker}\n{marker}: {marker}\nfolders:\n  /{marker}: {marker}\n  ~/{marker}: {marker}\n"
            ));
            put(&machine, Kind::State, &format!(
                "- root: /{marker}\n  account: {{claude: {marker}}}\n  source: {marker}\n- {marker}\n"
            ));
            for kind in KINDS {
                mode(&machine, &kind.file(), 0o666);
            }
            let files = files(&machine, &[]);
            let config = files.user_config();
            let allowed = files.allow_state();
            prop_assert!(matches!(config.status(), FileStatus::Ignored(_)));
            prop_assert!(matches!(allowed.status(), FileStatus::Ignored(_)));
            prop_assert!(config.value().default_name().is_none());
            prop_assert!(config.value().folders().is_empty());
            prop_assert!(allowed.value().entries().is_empty());
            prop_assert!(allowed.value().unresolved().is_empty());
            prop_assert!(allowed.value().ignored().is_empty());
            let lines: Vec<String> = config.warnings().iter().chain(allowed.warnings().iter())
                .map(ToString::to_string).collect();
            prop_assert_eq!(lines.len(), 2);
            prop_assert!(lines.iter().all(|line| !line.contains(&marker)), "{:?}", lines);
        }

        /// A name or a value that breaks its rule is dropped and never repeated, in a file that is
        /// trusted, whichever of the many places it stands.
        #[test]
        fn an_invalid_name_is_never_repeated_in_a_warning(marker in "[A-Z]{8,12}") {
            let machine = Machine::new();
            let config = config(&machine, &format!(
                "default_name: {marker}\nfolders:\n  /nowhere: {marker}\n  /elsewhere:\n    claude: {marker}\n"
            ));
            let allowed = state(&machine, &format!(
                "- root: /nowhere\n  account: {{claude: {marker}}}\n  source: s\n\
                 - root: relative\n  account: {{claude: a}}\n  source: {marker}\n\
                 - {marker}\n- root: /x\n  account: {marker}\n  source: s\n"
            ));
            prop_assert!(config.value().default_name().is_none());
            prop_assert!(allowed.value().entries().is_empty());
            let lines: Vec<String> = config.warnings().iter().chain(allowed.warnings().iter())
                .map(ToString::to_string).collect();
            prop_assert!(!lines.is_empty());
            prop_assert!(lines.iter().all(|line| !line.contains(&marker)), "{:?}", lines);
        }
    }
}
