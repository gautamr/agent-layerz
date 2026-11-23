//! A `path:` layer's directory is opened by the component walk and never reads in an account
//! folder (spec §6 "Layers in or around an account folder", TASK-52 AC #1; decision-27).

#[path = "support/machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use alz_core::Stripped;
    use alz_core::account::Relation;
    use alz_core::path::SafeRelPath;
    use alz_io::{Dir, ReadError, WalkError};

    use super::machine::Machine;

    const CAP: usize = 1024;

    fn layer_yaml() -> SafeRelPath {
        SafeRelPath::try_from("layer.yaml").expect("a safe path")
    }

    fn assert_refused(result: Result<Dir, WalkError>, relation: Relation) {
        match result {
            Err(WalkError::Refused(found)) => assert_eq!(found, relation),
            other => panic!("expected a refusal as {relation:?}, got {other:?}"),
        }
    }

    #[test]
    fn a_layer_inside_an_account_folder_is_refused_before_anything_in_it_is_looked_at() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-x/layer");
        machine.write("home/.claude-x/layer/layer.yaml", "name: x\n");
        let _locked = machine.lock_down("home/.claude-x");
        // Mode 000: a lookup of anything inside would fail with a permission error, not a refusal.
        for layer in [
            "home/.claude-x/layer",
            "home/.claude-x/layer/deeper",
            "home/.claude-x",
        ] {
            assert_refused(
                Dir::open(&machine.abs(layer), &machine.folders(None, None)),
                Relation::AccountFolder,
            );
        }
    }

    #[test]
    fn every_account_folder_name_is_refused() {
        let machine = Machine::new();
        for name in [
            ".claude",
            ".codex",
            ".claude-work",
            ".codex-work",
            ".CLAUDE-x",
        ] {
            machine.make_dirs(&format!("home/{name}"));
            assert_refused(
                Dir::open(
                    &machine.abs(&format!("home/{name}")),
                    &machine.folders(None, None),
                ),
                Relation::AccountFolder,
            );
        }
    }

    #[test]
    fn a_link_planted_in_the_repository_into_an_account_folder_is_refused_before_it_is_looked_at() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-x/layer");
        machine.write("home/.claude-x/layer/layer.yaml", "name: x\n");
        machine.link("repo/layer", "../home/.claude-x/layer");
        machine.link(
            "repo/absolute",
            machine.path("home/.claude-x").to_str().expect("text"),
        );
        let _locked = machine.lock_down("home/.claude-x");
        for layer in ["repo/layer", "repo/absolute", "repo/absolute/layer"] {
            assert_refused(
                Dir::open(&machine.abs(layer), &machine.folders(None, None)),
                Relation::AccountFolder,
            );
        }
    }

    #[test]
    fn a_layer_that_is_the_home_folder_or_above_it_is_refused() {
        let machine = Machine::new();
        machine.write("home/layer.yaml", "name: x\n");
        machine.write("layer.yaml", "name: x\n");
        for layer in ["home", "home/", "home/..", "other/../home"] {
            assert_refused(
                Dir::open(&machine.abs(layer), &machine.folders(None, None)),
                Relation::Home,
            );
        }
    }

    #[test]
    fn a_layer_that_holds_a_configured_folder_is_refused_and_one_inside_it_too() {
        let machine = Machine::new();
        machine.make_dirs("repo/accounts/claude/inner");
        machine.make_dirs("repo/accounts/codex");
        let folders = machine.folders(Some("repo/accounts/claude"), Some("repo/accounts/codex"));
        assert_refused(
            Dir::open(&machine.abs("repo/accounts"), &folders),
            Relation::Holds,
        );
        assert_refused(Dir::open(&machine.abs("repo"), &folders), Relation::Holds);
        let _locked = machine.lock_down("repo/accounts/claude");
        assert_refused(
            Dir::open(&machine.abs("repo/accounts/claude/inner"), &folders),
            Relation::Configured,
        );
        assert_refused(
            Dir::open(&machine.abs("repo/accounts/codex"), &folders),
            Relation::Configured,
        );
    }

    #[test]
    fn a_refusal_is_the_error_source_that_names_the_layer_and_never_the_path() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-x");
        let error = Dir::open(&machine.abs("home/.claude-x"), &machine.folders(None, None))
            .expect_err("refused");
        let line = error
            .into_layer_diagnostic(&Stripped::from_untrusted("path:../home/.claude-x"))
            .to_string();
        assert_eq!(
            line,
            "error [source] layer 'path:../home/.claude-x' touches an account folder or the home folder; alz never reads there"
        );
    }

    #[test]
    fn a_layer_outside_every_account_folder_is_opened_through_links_the_walk_follows() {
        let machine = Machine::new();
        machine.make_dirs("other/layer");
        machine.write("other/layer/layer.yaml", "name: x\n");
        machine.link("repo/alias", "../other/layer");
        machine.link("repo/two", "alias");
        let folders = machine.folders(None, None);
        for layer in [
            "other/layer",
            "other/layer/",
            "repo/alias",
            "repo/alias/",
            "repo/two",
            "repo/../repo/./two",
        ] {
            let dir = Dir::open(&machine.abs(layer), &folders).expect("opened");
            assert_eq!(
                dir.read_file(&layer_yaml(), CAP).expect("read"),
                b"name: x\n"
            );
        }
    }

    #[test]
    fn a_directory_that_is_missing_or_a_file_is_not_a_layer_directory() {
        let machine = Machine::new();
        machine.write("other/file", "x");
        let folders = machine.folders(None, None);
        assert!(matches!(
            Dir::open(&machine.abs("other/nowhere"), &folders),
            Err(WalkError::Missing)
        ));
        assert!(matches!(
            Dir::open(&machine.abs("other/file"), &folders),
            Err(WalkError::NotADirectory)
        ));
        assert!(matches!(
            Dir::open(&machine.abs("other/file/more"), &folders),
            Err(WalkError::NotADirectory)
        ));
    }

    #[test]
    fn a_cycle_of_links_ends_with_too_many_links() {
        let machine = Machine::new();
        machine.link("other/a", "b");
        machine.link("other/b", "a");
        assert!(matches!(
            Dir::open(&machine.abs("other/a"), &machine.folders(None, None)),
            Err(WalkError::TooManyLinks)
        ));
    }

    #[test]
    fn a_name_swapped_for_a_link_after_the_open_changes_nothing_the_handle_reads() {
        let machine = Machine::new();
        machine.make_dirs("other/layer");
        machine.write("other/layer/layer.yaml", "name: original\n");
        machine.make_dirs("other/evil");
        machine.write("other/evil/layer.yaml", "name: evil\n");
        let dir =
            Dir::open(&machine.abs("other/layer"), &machine.folders(None, None)).expect("opened");
        std::fs::rename(machine.path("other/layer"), machine.path("other/moved")).expect("moved");
        machine.link("other/layer", "evil");
        assert_eq!(
            dir.read_file(&layer_yaml(), CAP).expect("read"),
            b"name: original\n"
        );
    }

    #[test]
    fn a_link_inside_the_layer_is_never_followed_by_a_read() {
        let machine = Machine::new();
        machine.make_dirs("other/layer/sub");
        machine.make_dirs("other/secret");
        machine.write("other/secret/layer.yaml", "name: secret\n");
        machine.link("other/layer/alias", "../secret");
        machine.link("other/layer/layer.yaml", "../secret/layer.yaml");
        let dir =
            Dir::open(&machine.abs("other/layer"), &machine.folders(None, None)).expect("opened");
        for path in ["layer.yaml", "alias/layer.yaml"] {
            let path = SafeRelPath::try_from(path).expect("a safe path");
            assert!(matches!(
                dir.read_file(&path, CAP),
                Err(ReadError::NotRegular)
            ));
        }
    }
}
