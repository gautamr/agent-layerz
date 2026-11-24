//! `alz account which` (spec §10.3, TASK-55.7.10): the exit codes 0, 1 and 2, the warnings on
//! standard error only, and that nothing is fetched, run or written. Every run has its own home,
//! `XDG_*` folders and a `PATH` that holds only a stand-in `git` which records each call.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A repository in an isolated world: `home`, `config`, `state`, `cache` and `bin` (the
    /// stand-in `git`, which writes `git-calls` when it is run).
    struct World {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            for folder in ["home", "config", "state", "cache", "bin", "repo/.layer"] {
                scratch.make_dirs(folder).expect("made");
            }
            scratch
                .write(
                    "repo/layer.yaml",
                    "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
                )
                .expect("written");
            scratch
                .write(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
            scratch
                .write_executable(
                    "bin/git",
                    format!(
                        "#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n",
                        scratch.join("git-calls").display()
                    ),
                )
                .expect("a stand-in git");
            Self {
                repository: scratch.join("repo"),
                scratch,
            }
        }

        fn alz(&self, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(&self.repository)
                .env_clear()
                .env("PATH", self.scratch.join("bin"))
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .timeout(Duration::from_secs(60))
                .args(arguments);
            for (name, value) in extra {
                alz.env(name, value);
            }
            let output = alz.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }
    }

    #[test]
    fn it_exits_0_when_every_target_resolves_and_prints_the_report_on_standard_output() {
        let run = World::new().alz(&["account", "which"], &[]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .contains("claude: default (source: default) CLAUDE_CONFIG_DIR unset"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout
                .contains("codex: default (source: default) CODEX_HOME unset"),
            "{}",
            run.stdout
        );
        assert_eq!(run.stderr, "");
    }

    #[test]
    fn it_exits_1_when_a_named_account_fails_closed() {
        let run = World::new().alz(&["account", "which", "--use", "claude=work"], &[]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stdout.contains("claude: fails closed (source: use)"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("fix: alz account init work"),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn it_exits_2_with_nothing_on_standard_output_for_a_usage_error() {
        let world = World::new();
        for arguments in [
            vec!["account", "which", "--use", "claude=Bad_Name"],
            vec!["account", "which", "--use", "cursor=work"],
            vec!["account", "which", "--use", "claude"],
            vec!["account", "which", "--use", "claude=a", "--use", "claude=b"],
            vec!["account", "which", "stray"],
            vec!["account", "nothing"],
        ] {
            let run = world.alz(&arguments, &[]);
            assert_eq!(run.code, 2, "{arguments:?}: {}", run.stderr);
            assert_eq!(run.stdout, "", "{arguments:?}");
            assert_ne!(run.stderr, "", "{arguments:?}");
        }
    }

    #[test]
    fn a_warning_goes_to_standard_error_only_and_does_not_fail_the_command() {
        let run = World::new().alz(
            &["account", "which"],
            &[("CLAUDE_CONFIG_DIR", "/somewhere/else")],
        );
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr
                .contains("warning [account-foreign] CLAUDE_CONFIG_DIR was set outside alz"),
            "{}",
            run.stderr
        );
        assert!(!run.stdout.contains("warning ["), "{}", run.stdout);
    }

    #[test]
    fn it_runs_no_git_and_changes_no_file_in_an_allowed_repository() {
        let world = World::new();
        let real = world.scratch.real_path().expect("a real path");
        world.scratch.make_dirs("state/agent-layerz").expect("made");
        world
            .scratch
            .write(
                "state/agent-layerz/allowed.yaml",
                format!(
                    "- root: {root}\n  account:\n    claude: work\n  source: {root}/.layer\n",
                    root = real.join("repo").display()
                ),
            )
            .expect("written");
        world
            .scratch
            .set_mode("state/agent-layerz", 0o700)
            .expect("a private folder");
        world
            .scratch
            .set_mode("state/agent-layerz/allowed.yaml", 0o600)
            .expect("a private file");
        let before = read_tree(world.scratch.path()).expect("a tree");
        let run = world.alz(&["account", "which"], &[]);
        assert!(
            run.code == 0 || run.code == 1,
            "{}{}",
            run.stdout,
            run.stderr
        );
        assert!(
            run.stdout.contains("this repository is allowed"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("cache miss: layer.lock is missing"),
            "{}",
            run.stdout
        );
        assert!(!world.scratch.exists("git-calls"), "git was run");
        assert_eq!(
            read_tree(world.scratch.path()).expect("a tree"),
            before,
            "alz account which wrote"
        );
    }
}
