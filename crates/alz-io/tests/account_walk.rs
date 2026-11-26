//! The working directory, once; the root walk; the listing of the account folders (spec §7.13
//! "Paths", "Resolution", §9.1; decision-19, TASK-52, TASK-55.7.7). Every test uses a temporary
//! `HOME`. A folder whose mode is 000 makes any `lstat` or open inside it fail, so a result that
//! is not an error shows that nothing there was looked at.

#[path = "support/account_machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use alz_core::account::AccountFamily;
    use alz_core::account::{AccountFolders, AccountName};
    use alz_core::path::AbsolutePath;
    use alz_io::{
        AccountFolderNames, ManifestState, NoRoot, OnDisk, Place, PlaceLookup, ReadOnlyDir,
        Repository, WorkingDirectory, find_root,
    };
    use alz_test_support::running_as_root;
    use proptest::prelude::*;

    use super::machine::Machine;

    const STACK: &str = "stack:\n  - source: path:./.layer\n";

    fn folders(machine: &Machine) -> AccountFolders {
        AccountFolders::from_env(&machine.abs("home"), None, None)
    }

    fn working_directory(machine: &Machine, relative: &str) -> WorkingDirectory {
        WorkingDirectory::at(machine.abs(relative))
    }

    fn search(machine: &Machine, relative: &str) -> alz_io::RootSearch {
        let folders = folders(machine);
        let home = machine.abs("home");
        let files = ReadOnlyDir::open(Path::new("/")).expect("the root folder");
        find_root(
            working_directory(machine, relative).resolve(&folders),
            &folders,
            &[&home],
            &files,
        )
    }

    fn root_of(search: &alz_io::RootSearch) -> Option<(&AbsolutePath, &ManifestState)> {
        match search.repository() {
            Repository::Root(root) => Some((root.path(), root.manifest())),
            Repository::NoRoot(_) => None,
        }
    }

    fn lines(search: &alz_io::RootSearch) -> Vec<String> {
        search.warnings().iter().map(ToString::to_string).collect()
    }

    /// Counts the lookups it is asked for and passes each to the real file system.
    struct Counting(Cell<usize>);

    impl PlaceLookup for Counting {
        fn lookup(&self, directory: &AbsolutePath, folders: &AccountFolders) -> Place {
            self.0.set(self.0.get() + 1);
            OnDisk.lookup(directory, folders)
        }
    }

    #[test]
    fn the_working_directory_is_looked_up_once_and_resolved_once_for_the_whole_flow() {
        let machine = Machine::new();
        machine.make_dirs("repo/sub");
        machine.write("repo/layer.yaml", STACK);
        let process_lookups = Cell::new(0_usize);
        let working = WorkingDirectory::capture_with(|| {
            process_lookups.set(process_lookups.get() + 1);
            Ok(machine.path("repo/sub"))
        });
        let resolutions = Counting(Cell::new(0));
        let folders = folders(&machine);
        // `resolve_with` takes the directory by value; the place is what the later steps get.
        let place = working.resolve_with(&folders, &resolutions);
        let home = machine.abs("home");
        let files = ReadOnlyDir::open(Path::new("/")).expect("root folder");
        let found = find_root(place, &folders, &[&home], &files);
        let _listing = AccountFolderNames::list(&home);
        assert_eq!(process_lookups.get(), 1, "the process is asked once");
        assert_eq!(resolutions.0.get(), 1, "the walk resolves it once");
        assert_eq!(
            root_of(&found).map(|(path, _)| path.clone()),
            Some(machine.abs("repo"))
        );
    }

    #[test]
    fn a_directory_that_names_an_account_folder_or_home_as_written_makes_no_lookup() {
        let machine = Machine::new();
        for (relative, expected) in [
            ("home/.claude-work/sub", Place::InAccountFolder),
            ("home/.codex", Place::InAccountFolder),
            ("home", Place::AtOrAboveHome),
        ] {
            let resolutions = Counting(Cell::new(0));
            let place = working_directory(&machine, relative)
                .resolve_with(&folders(&machine), &resolutions);
            assert_eq!(place, expected, "{relative}");
            assert_eq!(resolutions.0.get(), 0, "{relative}");
        }
    }

    #[test]
    fn no_working_directory_and_a_missing_one_are_unresolvable() {
        let machine = Machine::new();
        let none = WorkingDirectory::capture_with(|| Err(std::io::Error::other("gone")));
        assert_eq!(none.resolve(&folders(&machine)), Place::Unresolvable);
        let missing = working_directory(&machine, "repo/not-there").resolve(&folders(&machine));
        assert_eq!(missing, Place::Unresolvable);
    }

    #[test]
    fn the_nearest_stack_manifest_is_the_root_and_a_layers_own_manifest_is_skipped() {
        let machine = Machine::new();
        machine.make_dirs("repo/inner/deep");
        machine.write("repo/layer.yaml", STACK);
        machine.write("repo/inner/layer.yaml", "name: a-layer\nkind: team\n");
        let found = search(&machine, "repo/inner/deep");
        let (root, manifest) = root_of(&found).expect("a root");
        assert_eq!(root, &machine.abs("repo"));
        assert!(matches!(manifest, ManifestState::Read(_)));
        assert_eq!(lines(&found), Vec::<String>::new());
        assert_eq!(
            found.known_directory(),
            Some(&machine.abs("repo/inner/deep"))
        );
    }

    #[test]
    fn a_nested_stack_manifest_wins_over_the_outer_one() {
        let machine = Machine::new();
        machine.make_dirs("repo/inner");
        machine.write("repo/layer.yaml", STACK);
        machine.write("repo/inner/layer.yaml", STACK);
        let found = search(&machine, "repo/inner");
        assert_eq!(
            root_of(&found).map(|(path, _)| path.clone()),
            Some(machine.abs("repo/inner"))
        );
    }

    #[test]
    fn the_walk_stops_at_the_file_system_root_with_no_repository() {
        let machine = Machine::new();
        machine.make_dirs("other/a/b");
        let found = search(&machine, "other/a/b");
        assert_eq!(found.repository(), &Repository::NoRoot(NoRoot::NotFound));
        assert_eq!(lines(&found), Vec::<String>::new());
    }

    #[test]
    fn the_walk_never_opens_a_layer_yaml_at_home_or_above_it() {
        let machine = Machine::new();
        machine.make_dirs("home/work/project");
        machine.write("home/layer.yaml", STACK);
        machine.write("layer.yaml", STACK);
        let found = search(&machine, "home/work/project");
        assert_eq!(found.repository(), &Repository::NoRoot(NoRoot::NotFound));
        if !running_as_root() {
            // Take every permission from the manifest at home: an open would be a warning.
            fs::set_permissions(
                machine.path("home/layer.yaml"),
                fs::Permissions::from_mode(0o000),
            )
            .expect("mode");
            let again = search(&machine, "home/work/project");
            fs::set_permissions(
                machine.path("home/layer.yaml"),
                fs::Permissions::from_mode(0o600),
            )
            .expect("restored");
            assert_eq!(again.repository(), &Repository::NoRoot(NoRoot::NotFound));
            assert_eq!(lines(&again), Vec::<String>::new());
        }
    }

    #[test]
    fn a_directory_at_home_is_no_repository_before_any_call() {
        let machine = Machine::new();
        machine.write("home/layer.yaml", STACK);
        let found = search(&machine, "home");
        assert_eq!(found.repository(), &Repository::NoRoot(NoRoot::NotFound));
        assert_eq!(found.known_directory(), None);
    }

    /// One unusable `layer.yaml` in `repo/inner`, with a valid root above it.
    fn assert_unusable_stops_the_walk(machine: &Machine, expected: &ManifestState, why: &str) {
        let found = search(machine, "repo/inner");
        let (root, manifest) = root_of(&found).expect("the nested directory is the root");
        assert_eq!(root, &machine.abs("repo/inner"), "{why}");
        assert_eq!(manifest, expected, "{why}");
        let shown = machine.path("repo/inner/layer.yaml");
        let text = lines(&found);
        assert_eq!(text.len(), 1, "{why}: {text:?}");
        assert!(
            text.first()
                .is_some_and(|line| line.starts_with("warning [account-manifest] ")
                    && line.contains(shown.to_str().expect("text"))
                    && line.ends_with("; the walk for the repository stops here")),
            "{why}: {text:?}"
        );
    }

    fn nested() -> Machine {
        let machine = Machine::new();
        machine.make_dirs("repo/inner");
        machine.write("repo/layer.yaml", STACK);
        machine
    }

    #[test]
    fn each_unusable_layer_yaml_stops_the_walk_with_one_warning_naming_the_file() {
        let machine = nested();
        // A link to a valid manifest.
        machine.write("repo/real.yaml", STACK);
        machine.link(
            "repo/inner/layer.yaml",
            machine.path("repo/real.yaml").to_str().expect("text"),
        );
        assert_unusable_stops_the_walk(&machine, &ManifestState::Unreadable, "a link");
        fs::remove_file(machine.path("repo/inner/layer.yaml")).expect("removed");

        machine.make_dirs("repo/inner/layer.yaml");
        assert_unusable_stops_the_walk(&machine, &ManifestState::Unreadable, "a folder");
        fs::remove_dir(machine.path("repo/inner/layer.yaml")).expect("removed");

        machine.write(
            "repo/inner/layer.yaml",
            &format!("stack: []\n# {}\n", "x".repeat(1_048_576)),
        );
        assert_unusable_stops_the_walk(&machine, &ManifestState::Unreadable, "over 1 MiB");

        machine.write("repo/inner/layer.yaml", "stack: [\n");
        assert_unusable_stops_the_walk(&machine, &ManifestState::Unreadable, "invalid YAML");

        let aliases = format!("stack: &s []\nmore: [{}]\n", vec!["*s"; 101].join(", "));
        machine.write("repo/inner/layer.yaml", &aliases);
        assert_unusable_stops_the_walk(
            &machine,
            &ManifestState::Unreadable,
            "past the alias limit",
        );

        machine.write("repo/inner/layer.yaml", "stack: 5\n");
        assert_unusable_stops_the_walk(&machine, &ManifestState::Invalid, "a schema error");

        if !running_as_root() {
            machine.write("repo/inner/layer.yaml", STACK);
            fs::set_permissions(
                machine.path("repo/inner/layer.yaml"),
                fs::Permissions::from_mode(0o000),
            )
            .expect("mode");
            assert_unusable_stops_the_walk(&machine, &ManifestState::Unreadable, "unreadable");
            fs::set_permissions(
                machine.path("repo/inner/layer.yaml"),
                fs::Permissions::from_mode(0o600),
            )
            .expect("restored");
        }
    }

    #[test]
    fn a_stack_manifest_that_is_a_file_with_a_valid_stack_is_read_without_a_warning() {
        let machine = nested();
        machine.write("repo/inner/layer.yaml", STACK);
        let found = search(&machine, "repo/inner");
        assert!(matches!(
            root_of(&found).map(|(_, manifest)| manifest),
            Some(ManifestState::Read(_))
        ));
        assert_eq!(lines(&found), Vec::<String>::new());
    }

    #[test]
    fn a_working_directory_through_a_link_into_an_account_folder_finds_no_root_and_opens_nothing() {
        if running_as_root() {
            return;
        }
        let machine = Machine::new();
        machine.make_dirs("home/.claude-work/project");
        machine.write("home/.claude-work/project/layer.yaml", STACK);
        machine.make_dirs("repo");
        machine.link(
            "repo/plant",
            machine
                .path("home/.claude-work/project")
                .to_str()
                .expect("text"),
        );
        let account = machine.path("home/.claude-work");
        fs::set_permissions(&account, fs::Permissions::from_mode(0o000)).expect("mode");
        let found = search(&machine, "repo/plant");
        fs::set_permissions(&account, fs::Permissions::from_mode(0o700)).expect("restored");
        assert_eq!(
            found.repository(),
            &Repository::NoRoot(NoRoot::AccountFolder)
        );
        assert_eq!(lines(&found), Vec::<String>::new());
    }

    #[test]
    fn a_working_directory_in_a_configured_folder_finds_no_root_and_opens_nothing_there() {
        let machine = Machine::new();
        machine.make_dirs("other/cfg/sub");
        machine.write("other/cfg/layer.yaml", STACK);
        let configured = machine.abs("other/cfg");
        let folders = AccountFolders::from_env(&machine.abs("home"), Some(&configured), None)
            .with_real_configured(&configured);
        let home = machine.abs("home");
        let files = ReadOnlyDir::open(Path::new("/")).expect("root");
        let place = working_directory(&machine, "other/cfg/sub").resolve(&folders);
        assert_eq!(place, Place::InConfiguredFolder(configured.clone()));
        let found = find_root(place, &folders, &[&home], &files);
        assert_eq!(
            found.repository(),
            &Repository::NoRoot(NoRoot::ConfiguredFolder(configured))
        );
    }

    #[test]
    fn a_directory_that_only_holds_a_configured_folder_stays_a_root() {
        let machine = Machine::new();
        machine.make_dirs("repo/cfg");
        machine.write("repo/layer.yaml", STACK);
        let configured = machine.abs("repo/cfg");
        let folders = AccountFolders::from_env(&machine.abs("home"), Some(&configured), None);
        let home = machine.abs("home");
        let files = ReadOnlyDir::open(Path::new("/")).expect("root");
        let place = working_directory(&machine, "repo").resolve(&folders);
        assert_eq!(place, Place::Directory(machine.abs("repo")));
        let found = find_root(place, &folders, &[&home], &files);
        assert_eq!(
            root_of(&found).map(|(path, _)| path.clone()),
            Some(machine.abs("repo"))
        );
    }

    /// Where a planted link in the repository can point: names inside account folders.
    fn planted_target() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just(".claude-work"),
            Just(".claude-work/a/b"),
            Just(".codex-team/x"),
            Just(".claude"),
            Just(".codex/sessions"),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(12))]

        /// TASK-52 #4, account side: neither the place nor the root walk looks at a name inside
        /// an account folder, even through a link planted in the repository. The account folders
        /// have mode 000, so any `lstat`, readlink or open inside one would fail the lookup.
        #[test]
        fn nothing_inside_an_account_folder_is_looked_at_through_a_planted_link(
            target in planted_target(),
            below in prop::collection::vec("[a-z]{1,4}", 0..3),
        ) {
            prop_assume!(!running_as_root());
            let machine = Machine::new();
            let account_folders = [".claude-work", ".codex-team", ".claude", ".codex"];
            for folder in account_folders {
                machine.make_dirs(&format!("home/{folder}/a/b"));
                machine.write(&format!("home/{folder}/layer.yaml"), STACK);
            }
            machine.make_dirs("repo");
            machine.link("repo/plant", machine.path(&format!("home/{target}")).to_str().expect("text"));
            let cwd = std::iter::once("repo/plant".to_owned())
                .chain(below)
                .collect::<Vec<_>>()
                .join("/");
            for folder in account_folders {
                fs::set_permissions(machine.path(&format!("home/{folder}")), fs::Permissions::from_mode(0o000)).expect("mode");
            }
            let found = search(&machine, &cwd);
            for folder in account_folders {
                fs::set_permissions(machine.path(&format!("home/{folder}")), fs::Permissions::from_mode(0o700)).expect("restored");
            }
            prop_assert_eq!(found.repository(), &Repository::NoRoot(NoRoot::AccountFolder));
            prop_assert_eq!(lines(&found), Vec::<String>::new());
        }
    }

    #[test]
    fn the_listing_of_home_reads_names_only_and_skips_the_blocked_folder() {
        let machine = Machine::new();
        for folder in [
            "home/.claude-alpha",
            "home/.claude-alz-blocked",
            "home/.codex-beta",
            "home/.claude-Upper",
        ] {
            machine.make_dirs(folder);
            machine.write(&format!("{folder}/secret"), "login");
        }
        machine.make_dirs("other/target");
        machine.link(
            "home/.codex-linked",
            machine.path("other/target").to_str().expect("text"),
        );
        machine.write("home/.claude-afile", "not a folder");
        machine.make_dirs("home/.codex-alz-blocked");
        machine.make_dirs("home/.claude");
        let writable = !running_as_root();
        if writable {
            for folder in ["home/.claude-alpha", "home/.codex-beta"] {
                fs::set_permissions(machine.path(folder), fs::Permissions::from_mode(0o000))
                    .expect("mode");
            }
        }
        let listed = AccountFolderNames::list(&machine.abs("home"));
        if writable {
            for folder in ["home/.claude-alpha", "home/.codex-beta"] {
                fs::set_permissions(machine.path(folder), fs::Permissions::from_mode(0o700))
                    .expect("restored");
            }
        }
        let claude: Vec<&str> = listed
            .names(AccountFamily::Claude)
            .map(AccountName::as_str)
            .collect();
        let codex: Vec<&str> = listed
            .names(AccountFamily::Codex)
            .map(AccountName::as_str)
            .collect();
        assert_eq!(claude, ["alpha"]);
        assert_eq!(codex, ["beta", "linked"]);
        let blocked = AccountName::try_from("alz-blocked").expect("a name");
        assert!(!listed.exists(AccountFamily::Claude, &blocked));
        assert!(!listed.exists(AccountFamily::Codex, &blocked));
    }

    #[test]
    fn a_home_that_cannot_be_listed_has_no_accounts() {
        let machine = Machine::new();
        let listed = AccountFolderNames::list(&machine.abs("home/not-there"));
        assert_eq!(listed.names(AccountFamily::Claude).count(), 0);
        assert_eq!(listed.names(AccountFamily::Codex).count(), 0);
    }
}
