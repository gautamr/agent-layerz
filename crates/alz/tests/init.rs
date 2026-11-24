//! `alz init` (spec §10, §10.3): what it creates, byte for byte, and what it never touches. Each
//! test runs the binary in a scratch repository folder whose home folder is another folder, so
//! nothing reads or writes the real home.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const REPO_LAYER: &str = "name: repo\nkind: repo\nvisibility: private\nversion: 0.1.0\n";
    const MISE: &str = "[tasks.\"alz:build\"]\nrun = \"alz build\"\n\n[tasks.\"alz:check\"]\nrun = \"alz check\"\n\n[tasks.\"alz:diff\"]\nrun = \"alz diff\"\n\n[tasks.\"alz:doctor\"]\nrun = \"alz doctor\"\n";
    const NEXT_REPO: &str =
        "next: run `alz build --adopt` to take over existing agent config, or `alz build`\n";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A scratch folder with `repo` (where alz runs), `home` and `outside` (what no write may
    /// reach).
    struct Place {
        scratch: ScratchDir,
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            for name in ["repo", "home", "outside"] {
                scratch.make_dir(name).expect("made");
            }
            Self { scratch }
        }

        fn alz(&self, arguments: &[&str]) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            let output = command
                .current_dir(self.scratch.join("repo"))
                .env("HOME", self.scratch.join("home"))
                .timeout(Duration::from_secs(30))
                .args(arguments)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn text(&self, relative: &str) -> Option<String> {
            self.scratch.read_text(format!("repo/{relative}")).ok()
        }

        fn files(&self, folder: &str) -> Vec<String> {
            self.scratch
                .list(folder)
                .into_iter()
                .map(|name| format!("{folder}/{name}"))
                .flat_map(|path| {
                    if self.scratch.is_dir(&path) {
                        self.files(&path)
                    } else {
                        vec![path]
                    }
                })
                .collect()
        }
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let mut command = Command::new("git");
        command
            .current_dir(directory)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "tag.gpgsign=false",
            ])
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .assert()
            .success();
    }

    /// A hub whose `layer/` is committed and carries the given tags, newest last.
    fn hub_with_tags(place: &Place, tags: &[&str]) {
        place.scratch.make_dirs("hub/layer").expect("made");
        place
            .scratch
            .write(
                "hub/layer/layer.yaml",
                "name: client\nkind: client\nvisibility: private\n",
            )
            .expect("written");
        let hub = place.scratch.join("hub");
        git(&hub, &["init", "--quiet", "--initial-branch=main"]);
        git(&hub, &["add", "."]);
        git(&hub, &["commit", "--quiet", "-m", "client layer"]);
        for tag in tags {
            git(&hub, &["tag", tag]);
        }
    }

    #[test]
    fn a_repository_gets_the_scaffold_files_and_the_created_lines() {
        let place = Place::new();
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(
            run.stdout,
            format!(
                "created layer.yaml\ncreated .layer/layer.yaml\ncreated .layer/instructions/50-repo.md\ncreated mise.toml\n{NEXT_REPO}"
            )
        );
        assert_eq!(place.text(".layer/layer.yaml").as_deref(), Some(REPO_LAYER));
        assert_eq!(place.text("mise.toml").as_deref(), Some(MISE));
        assert_eq!(
            place.text(".layer/instructions/50-repo.md").as_deref(),
            Some("- Describe this repo's commands, paths and contracts here.\n")
        );
        assert_eq!(
            place.text("layer.yaml").as_deref(),
            Some(
                "stack:\n  # Most general first, for example:\n  # - source: github:<you>/agent-layerz-personal@v1.0.0\n  - source: path:./.layer   # the repo layer, always last\ntargets: [claude]\n# workspace:\n#   hub: ../hub             # granted to agents as an additional directory\n# protected_paths: []       # extra globs the build never touches\nmcp:\n  enable: []\nbudgets:\n  instructions_max_lines: 400\noptions:\n  commit_generated: true\n  strict_slots: true\n"
            )
        );
        assert_eq!(
            place.scratch.mode("repo/layer.yaml").expect("a mode") & 0o777,
            0o644
        );
    }

    #[test]
    fn the_name_of_a_repository_is_its_folders_name_made_into_a_layer_name() {
        let place = Place::new();
        place.scratch.make_dir("My_Repo").expect("made");
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(place.scratch.join("My_Repo"))
            .env("HOME", place.scratch.join("home"))
            .args(["init"])
            .output()
            .expect("alz runs");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            place
                .scratch
                .read_text("My_Repo/.layer/layer.yaml")
                .ok()
                .as_deref(),
            Some("name: my-repo\nkind: repo\nvisibility: private\nversion: 0.1.0\n")
        );
    }

    #[test]
    fn stack_options_are_written_as_path_pinned_path_and_github_sources() {
        let place = Place::new();
        let run = place.alz(&[
            "init",
            "--name",
            "repo",
            "--stack",
            "../nowhere",
            "--stack",
            "path:../hub/layer@layer-v1.0.0",
            "--stack",
            "github:me/agent-layerz-personal@v1.2.0",
            "--hub",
            "../hub",
        ]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "note [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it\n"
            )
        );
        let manifest = place.text("layer.yaml").expect("a manifest");
        assert!(
            manifest.starts_with(
                "stack:\n  - source: path:../nowhere\n  - source: path:../hub/layer\n    ref: layer-v1.0.0\n  - source: github:me/agent-layerz-personal@v1.2.0\n  - source: path:./.layer   # the repo layer, always last\ntargets: [claude]\nworkspace:\n  hub: ../hub             # granted"
            ),
            "{manifest}"
        );
    }

    #[test]
    fn a_refused_source_is_a_warning_and_not_the_note_about_a_stack_not_resolved_yet() {
        let place = Place::new();
        let run = place.alz(&["init", "--name", "repo", "--stack", "git:/srv/a/../b@v1"]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "warning [source] 'git:/srv/a/../b@v1': a local git: path must not contain '..' segments; use a path without them\n"
            )
        );
    }

    #[test]
    fn a_source_not_there_yet_adds_the_note_after_the_warnings_of_the_other_problems() {
        let place = Place::new();
        let run = place.alz(&[
            "init",
            "--name",
            "repo",
            "--stack",
            "../nowhere",
            "--stack",
            "git:/srv/a/../b@v1",
        ]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "warning [source] 'git:/srv/a/../b@v1': a local git: path must not contain '..' segments; use a path without them\nnote [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it\n"
            )
        );
    }

    #[test]
    fn a_layer_gets_the_scaffold_files_with_the_public_default() {
        let place = Place::new();
        let run = place.alz(&["init", "--kind", "layer", "--name", "x.y"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(
            run.stdout,
            "created layer.yaml\ncreated instructions/00-core.md\nnext: run `alz lint`\n"
        );
        assert_eq!(
            place.text("layer.yaml").as_deref(),
            Some("name: x.y\nkind: layer\nvisibility: public\nversion: 0.1.0\n")
        );
        assert_eq!(
            place.text("instructions/00-core.md").as_deref(),
            Some("- Write the standards of this layer here.\n")
        );
        let confidential = Place::new();
        confidential.alz(&[
            "init",
            "--kind",
            "layer",
            "--name",
            "x",
            "--visibility",
            "confidential",
        ]);
        assert_eq!(
            confidential.text("layer.yaml").as_deref(),
            Some("name: x\nkind: layer\nvisibility: confidential\nversion: 0.1.0\n")
        );
    }

    #[test]
    fn an_existing_layer_yaml_stops_init_and_nothing_is_written() {
        let place = Place::new();
        place
            .scratch
            .write("repo/layer.yaml", "mine\n")
            .expect("written");
        for arguments in [&["init"][..], &["init", "--kind", "layer"][..]] {
            let run = place.alz(arguments);
            assert_eq!((run.code, run.stdout.as_str()), (1, ""));
            assert_eq!(run.stderr, "error [init] layer.yaml already exists here\n");
        }
        assert_eq!(place.files("repo"), ["repo/layer.yaml"]);
        assert_eq!(place.text("layer.yaml").as_deref(), Some("mine\n"));
    }

    #[test]
    fn a_link_at_layer_yaml_counts_as_existing_and_its_target_is_untouched() {
        let place = Place::new();
        place
            .scratch
            .make_link("repo/layer.yaml", "../outside/target.yaml")
            .expect("linked");
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!(run.code, 1);
        assert_eq!(run.stderr, "error [init] layer.yaml already exists here\n");
        assert!(!place.scratch.exists("outside/target.yaml"));
        assert_eq!(place.files("outside"), Vec::<String>::new());
    }

    #[test]
    fn an_existing_repo_layer_and_mise_config_are_kept_and_the_tasks_are_printed() {
        let place = Place::new();
        place.scratch.make_dir("repo/.layer").expect("made");
        place
            .scratch
            .write("repo/.layer/layer.yaml", "name: mine\n")
            .expect("written");
        place
            .scratch
            .write("repo/mise.toml", "# mine\n")
            .expect("written");
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "warning [layer] path:./.layer:layer.yaml:1:1: kind is required\nwarning [layer] path:./.layer:layer.yaml:1:1: visibility is required\n"
            )
        );
        assert_eq!(
            run.stdout,
            format!(
                "add these tasks to your mise config:\n\n{MISE}\ncreated layer.yaml\n{NEXT_REPO}"
            )
        );
        assert_eq!(
            place.text(".layer/layer.yaml").as_deref(),
            Some("name: mine\n")
        );
        assert_eq!(place.text("mise.toml").as_deref(), Some("# mine\n"));
        assert!(!place.scratch.exists("repo/.layer/instructions"));
    }

    #[test]
    fn a_dot_mise_toml_counts_as_a_mise_config() {
        let place = Place::new();
        place
            .scratch
            .write("repo/.mise.toml", "# mine\n")
            .expect("written");
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!(run.code, 0);
        assert!(
            run.stdout
                .starts_with("add these tasks to your mise config:")
        );
        assert!(!place.scratch.exists("repo/mise.toml"));
    }

    #[test]
    fn a_link_for_the_repo_layer_is_not_followed_and_nothing_is_written_through_it() {
        let place = Place::new();
        place
            .scratch
            .make_link("repo/.layer", "../outside")
            .expect("linked");
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "warning [layer] missing layer.yaml in layer 'path:./.layer'\n"
            )
        );
        assert_eq!(place.files("outside"), Vec::<String>::new());
        assert!(place.scratch.exists("repo/layer.yaml"));
    }

    #[test]
    fn a_link_in_a_folder_init_must_make_is_refused_and_nothing_is_written_through_it() {
        let place = Place::new();
        place.scratch.make_dir("repo/.layer").expect("made");
        place
            .scratch
            .make_link("repo/.layer/instructions", "../../outside")
            .expect("linked");
        let run = place.alz(&["init", "--name", "repo"]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.starts_with("error [init] "), "{}", run.stderr);
        assert_eq!(place.files("outside"), Vec::<String>::new());
    }

    #[test]
    fn a_name_that_is_not_a_layer_name_is_an_error_and_nothing_is_written() {
        let place = Place::new();
        let run = place.alz(&["init", "--name", "Bad Name"]);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""));
        assert_eq!(
            run.stderr,
            "error [init] 'Bad Name' is not a valid layer name; pass --name\n"
        );
        assert_eq!(place.files("repo"), Vec::<String>::new());
    }

    #[test]
    fn a_visibility_outside_the_three_is_a_usage_error() {
        let place = Place::new();
        let run = place.alz(&["init", "--visibility", "secret"]);
        assert_eq!((run.code, run.stdout.as_str()), (2, ""));
        assert_ne!(run.stderr, "");
        assert_eq!(place.files("repo"), Vec::<String>::new());
    }

    #[test]
    fn a_stack_value_that_is_not_a_source_is_refused_and_never_printed() {
        let place = Place::new();
        let run = place.alz(&["init", "--name", "repo", "--stack", "github:me/token-x"]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.starts_with("error [init] a --stack value "));
        assert!(!run.stderr.contains("token-x"));
        assert_eq!(place.files("repo"), Vec::<String>::new());
    }

    #[test]
    fn the_hub_layer_is_pinned_to_its_newest_layer_tag_in_version_order() {
        let place = Place::new();
        hub_with_tags(
            &place,
            &["layer-v0.1.0", "layer-v0.10.0", "layer-v0.9.0", "other"],
        );
        let run = place.alz(&[
            "init",
            "--name",
            "repo",
            "--stack",
            "../personal",
            "--hub",
            "../hub",
        ]);
        assert_eq!(
            (run.code, run.stderr.as_str()),
            (
                0,
                "note [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it\n"
            )
        );
        assert!(
            run.stdout
                .starts_with("using the hub's client layer path:../hub/layer at layer-v0.10.0\n"),
            "{}",
            run.stdout
        );
        let manifest = place.text("layer.yaml").expect("a manifest");
        assert!(
            manifest.starts_with(
                "stack:\n  - source: path:../personal\n  - source: path:../hub/layer\n    ref: layer-v0.10.0\n  - source: path:./.layer"
            ),
            "{manifest}"
        );
        assert!(manifest.contains("workspace:\n  hub: ../hub  "));
    }

    #[test]
    fn a_hub_layer_with_no_tag_is_added_without_a_pin() {
        let place = Place::new();
        hub_with_tags(&place, &[]);
        let run = place.alz(&["init", "--name", "repo", "--hub", "../hub"]);
        assert_eq!(run.code, 0);
        assert!(
            run.stdout
                .starts_with("using the hub's client layer path:../hub/layer\n")
        );
        assert!(place.text("layer.yaml").is_some_and(|manifest| {
            manifest.contains("  - source: path:../hub/layer\n  - source: path:./.layer")
        }));
    }

    #[test]
    fn a_newest_hub_tag_that_is_not_a_git_ref_is_refused_before_anything_is_written() {
        let place = Place::new();
        hub_with_tags(&place, &["layer-v1.0.0", "layer-v1@2"]);
        let run = place.alz(&["init", "--name", "repo", "--hub", "../hub"]);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""));
        assert_eq!(
            run.stderr,
            "error [init] the newest layer-v* tag of the hub is not a valid git ref\n"
        );
        assert_eq!(place.files("repo"), Vec::<String>::new());
    }

    #[test]
    fn a_hub_without_a_client_layer_only_sets_the_workspace() {
        let place = Place::new();
        let run = place.alz(&["init", "--name", "repo", "--hub", "../hub"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert!(!run.stdout.contains("using the hub"));
        assert!(
            place
                .text("layer.yaml")
                .is_some_and(|manifest| manifest.contains("workspace:\n  hub: ../hub  ")
                    && !manifest.contains("hub/layer"))
        );
    }
}
