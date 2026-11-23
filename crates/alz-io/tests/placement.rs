//! Where a layer lies in relation to the repository (spec §7.12 "Who may declare it"): a path
//! inside, a case variant of one and a link into it count as inside; remotes and paths outside
//! count as outside.

#[path = "support/machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use alz_core::merge::Placement;
    use alz_core::stack::Source;
    use alz_io::{RepositoryPaths, placement_of_directory, placement_of_source};

    use super::machine::Machine;

    /// A machine whose `repo` is the repository, with the layers `repo/inner` and `other/outer`.
    fn machine() -> (Machine, RepositoryPaths) {
        let machine = Machine::new();
        machine.make_dirs("repo/inner");
        machine.make_dirs("other/outer");
        for layer in ["repo/inner", "other/outer"] {
            machine.write(
                &format!("{layer}/layer.yaml"),
                "name: layer\nkind: x\nvisibility: private\n",
            );
        }
        let repository = RepositoryPaths::new(machine.abs("repo"), machine.abs("repo"));
        (machine, repository)
    }

    fn directory(machine: &Machine, repository: &RepositoryPaths, relative: &str) -> Placement {
        placement_of_directory(
            &machine.abs(relative),
            repository,
            &machine.folders(None, None),
        )
    }

    fn source(machine: &Machine, repository: &RepositoryPaths, text: &str) -> Placement {
        placement_of_source(
            &Source::try_from(text).expect("a source"),
            repository,
            &machine.folders(None, None),
        )
    }

    /// Whether this machine's scratch folder ignores case in file names.
    fn ignores_case(machine: &Machine) -> bool {
        machine.path("REPO").exists()
    }

    #[test]
    fn a_directory_inside_the_repository_is_inside() {
        let (machine, repository) = machine();
        assert_eq!(
            directory(&machine, &repository, "repo/inner"),
            Placement::Inside
        );
        assert_eq!(directory(&machine, &repository, "repo"), Placement::Inside);
    }

    #[test]
    fn a_directory_outside_the_repository_is_outside() {
        let (machine, repository) = machine();
        assert_eq!(
            directory(&machine, &repository, "other/outer"),
            Placement::Outside
        );
    }

    #[test]
    fn a_folder_that_only_starts_with_the_repository_name_is_outside() {
        let (machine, repository) = machine();
        machine.make_dirs("repo-other/layer");
        assert_eq!(
            directory(&machine, &repository, "repo-other/layer"),
            Placement::Outside
        );
    }

    #[test]
    fn a_link_outside_the_repository_that_points_into_it_is_inside() {
        let (machine, repository) = machine();
        machine.link(
            "other/link",
            machine.path("repo/inner").to_str().expect("text"),
        );
        assert_eq!(
            directory(&machine, &repository, "other/link"),
            Placement::Inside
        );
    }

    #[test]
    fn a_link_inside_the_repository_that_points_outside_is_inside_as_written() {
        let (machine, repository) = machine();
        machine.link(
            "repo/ext",
            machine.path("other/outer").to_str().expect("text"),
        );
        assert_eq!(
            directory(&machine, &repository, "repo/ext"),
            Placement::Inside
        );
    }

    #[test]
    fn a_case_variant_of_a_path_inside_the_repository_is_inside() {
        let (machine, repository) = machine();
        // On a case-sensitive file system the variant names nothing, and a name that cannot be
        // opened counts as inside as well; on a case-insensitive one it is the folder itself.
        assert_eq!(
            directory(&machine, &repository, "REPO/inner"),
            Placement::Inside
        );
        if ignores_case(&machine) {
            assert_eq!(
                directory(&machine, &repository, "Repo/INNER"),
                Placement::Inside
            );
        }
    }

    #[test]
    fn a_directory_that_does_not_exist_counts_as_inside() {
        let (machine, repository) = machine();
        assert_eq!(
            directory(&machine, &repository, "other/missing"),
            Placement::Inside
        );
    }

    #[test]
    fn a_directory_that_cannot_be_opened_counts_as_inside() {
        let (machine, repository) = machine();
        machine.make_dirs("other/closed/layer");
        let _locked = machine.lock_down("other/closed");
        assert_eq!(
            directory(&machine, &repository, "other/closed/layer"),
            Placement::Inside
        );
    }

    #[test]
    fn a_path_source_is_read_against_the_repository_root() {
        let (machine, repository) = machine();
        assert_eq!(
            source(&machine, &repository, "path:./inner"),
            Placement::Inside
        );
        assert_eq!(
            source(&machine, &repository, "path:../other/outer"),
            Placement::Outside
        );
    }

    #[test]
    fn a_github_source_is_outside() {
        let (machine, repository) = machine();
        assert_eq!(
            source(&machine, &repository, "github:owner/layers@v1"),
            Placement::Outside
        );
    }

    #[test]
    fn a_git_remote_is_outside_unless_a_folder_of_that_name_is_in_the_repository() {
        let (machine, repository) = machine();
        let remote = "git:host.invalid:client.git@v1";
        assert_eq!(source(&machine, &repository, remote), Placement::Outside);
        assert_eq!(
            source(
                &machine,
                &repository,
                "git:https://example.invalid/o/r.git@v1"
            ),
            Placement::Outside
        );
        // `git clone` tries the local path of that name first (§7.12).
        machine.make_dirs("repo/host.invalid:client.git");
        assert_eq!(source(&machine, &repository, remote), Placement::Inside);
    }

    #[test]
    fn a_local_git_path_is_placed_by_its_canonical_path() {
        let (machine, repository) = machine();
        machine.make_dirs("repo/identity.git");
        machine.make_dirs("other/outside.git");
        assert_eq!(
            source(&machine, &repository, "git:./identity.git@v1"),
            Placement::Inside
        );
        assert_eq!(
            source(&machine, &repository, "git:../other/outside.git@v1"),
            Placement::Inside,
            "a `..` segment is a source error, which counts as inside"
        );
        machine.link(
            "other/via-link",
            machine.path("repo/identity.git").to_str().expect("text"),
        );
        let absolute = format!("git:{}@v1", machine.path("other/via-link").display());
        assert_eq!(source(&machine, &repository, &absolute), Placement::Inside);
        let outside = format!("git:{}@v1", machine.path("other/outside.git").display());
        assert_eq!(source(&machine, &repository, &outside), Placement::Outside);
    }
}
