//! The account note of `alz build` and `alz check` (spec §7.13 "Trust", §10.3 "The account
//! note", TASK-55.7.14): printed on standard error only for a person at a terminal (a
//! pseudo-terminal from `pty-run`, found next to the test executable), never when standard input
//! is a pipe or an agent-session marker is set, and never a change to standard output, a
//! generated file or the exit code. Every run has its own home and `XDG_*` folders.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use alz_core::account::AgentSessionMarker;
    use alz_test_support::{ScratchDir, pty_run_executable, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const NOTE: &str = "note [account-allow] the layer \"client\" names claude=client-a; it applies only after you run: alz account allow";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A repository whose stack holds a personal layer, a client layer that names an account and
    /// the repository's own layer, in an isolated world.
    struct World {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            for folder in [
                "home",
                "config",
                "state",
                "cache",
                "repo/.layer",
                "personal/instructions",
                "hub",
            ] {
                scratch.make_dirs(folder).expect("made");
            }
            for (path, text) in [
                (
                    "personal/layer.yaml",
                    "name: me\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
                ),
                ("personal/instructions/10-x.md", "- be brief.\n"),
                (
                    "hub/layer.yaml",
                    "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-a\n",
                ),
                (
                    "repo/layer.yaml",
                    "stack:\n  - source: path:../personal\n  - source: path:../hub\n  - source: path:./.layer\ntargets: [claude]\n",
                ),
                (
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                ),
            ] {
                scratch.write(path, text).expect("written");
            }
            Self {
                repository: scratch.join("repo"),
                scratch,
            }
        }

        /// A command with this world's environment and nothing of the test's own.
        fn command(&self, program: &std::path::Path) -> Command {
            let mut command = Command::new(program);
            command
                .current_dir(&self.repository)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .timeout(Duration::from_secs(60));
            command
        }

        fn finish(mut command: Command) -> Run {
            let output = command.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        /// `alz <arguments>` with standard input a pipe, so nobody is at a terminal.
        fn piped(&self, arguments: &[&str]) -> Run {
            let program = cargo_bin_cmd!("alz");
            let mut command = self.command(program.get_program().as_ref());
            command.args(arguments);
            Self::finish(command)
        }

        /// `alz <arguments>` on a pseudo-terminal, with `extra` set in its environment.
        fn at_terminal(&self, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
            let program = cargo_bin_cmd!("alz");
            let driver = pty_run_executable().expect("pty-run is built with the workspace");
            let mut command = self.command(&driver);
            command
                .arg("--")
                .arg(program.get_program())
                .args(arguments)
                .envs(extra.iter().copied());
            Self::finish(command)
        }

        /// Records this repository as allowed for the account the client layer names.
        fn allow(&self) {
            let root = self.scratch.real_path().expect("a real path");
            self.scratch
                .make_dirs("state/agent-layerz")
                .expect("made the state folder");
            self.scratch
                .set_mode("state/agent-layerz", 0o700)
                .expect("private");
            self.scratch
                .write(
                    "state/agent-layerz/allowed.yaml",
                    format!(
                        "- root: {root}/repo\n  account:\n    claude: client-a\n  source: {root}/hub\n",
                        root = root.display()
                    ),
                )
                .expect("written");
            self.scratch
                .set_mode("state/agent-layerz/allowed.yaml", 0o600)
                .expect("private");
        }
    }

    fn notes(stderr: &str) -> Vec<&str> {
        stderr
            .lines()
            .filter(|line| line.starts_with("note [account-allow]"))
            .collect()
    }

    #[test]
    fn build_prints_the_note_at_a_terminal_and_changes_nothing_else() {
        let world = World::new();
        let quiet = world.piped(&["build"]);
        assert_eq!(quiet.code, 0, "{}", quiet.stderr);
        assert_eq!(notes(&quiet.stderr), Vec::<&str>::new());
        let tree = read_tree(&world.repository).expect("read");
        let again = world.piped(&["build"]);

        let at_terminal = world.at_terminal(&["build"], &[]);
        assert_eq!(at_terminal.code, 0, "{}", at_terminal.stderr);
        assert_eq!(notes(&at_terminal.stderr), [NOTE]);
        assert_eq!(at_terminal.stdout, again.stdout);
        assert_eq!(read_tree(&world.repository).expect("read"), tree);
    }

    #[test]
    fn check_prints_the_note_at_a_terminal_without_changing_its_exit_code() {
        let world = World::new();
        assert_eq!(world.piped(&["build"]).code, 0);
        let piped = world.piped(&["check"]);
        let at_terminal = world.at_terminal(&["check"], &[]);
        assert_eq!(at_terminal.code, 0, "{}", at_terminal.stderr);
        assert_eq!(piped.code, 0);
        assert_eq!(notes(&at_terminal.stderr), [NOTE]);
        assert_eq!(notes(&piped.stderr), Vec::<&str>::new());
        assert_eq!(at_terminal.stdout, piped.stdout);
    }

    #[test]
    fn no_note_is_printed_with_any_agent_session_marker() {
        let world = World::new();
        assert_eq!(world.piped(&["build"]).code, 0);
        for marker in AgentSessionMarker::ALL {
            for command in ["build", "check"] {
                let run = world.at_terminal(&[command], &[(marker.name(), "")]);
                assert_eq!(
                    run.code,
                    0,
                    "{command} with {}: {}",
                    marker.name(),
                    run.stderr
                );
                assert!(
                    !run.stderr.contains("alz account allow"),
                    "{command} with {}: {}",
                    marker.name(),
                    run.stderr
                );
            }
        }
    }

    #[test]
    fn no_note_is_printed_once_the_repository_is_allowed() {
        let world = World::new();
        assert_eq!(world.piped(&["build"]).code, 0);
        world.allow();
        for command in ["build", "check"] {
            let run = world.at_terminal(&[command], &[]);
            assert_eq!(run.code, 0, "{}", run.stderr);
            assert_eq!(notes(&run.stderr), Vec::<&str>::new(), "{command}");
        }
    }

    #[test]
    fn a_changed_declaration_names_both_maps_and_asks_to_allow_again() {
        let world = World::new();
        assert_eq!(world.piped(&["build"]).code, 0);
        world.allow();
        world
            .scratch
            .write(
                "hub/layer.yaml",
                "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-c\n",
            )
            .expect("written");
        assert_eq!(world.piped(&["build"]).code, 0);
        let run = world.at_terminal(&["build"], &[]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        let [line] = notes(&run.stderr)[..] else {
            panic!("one note expected: {}", run.stderr)
        };
        assert!(line.contains("was claude=client-a from"), "{line}");
        assert!(line.contains("now claude=client-c from"), "{line}");
        assert!(line.ends_with("; run: alz account allow"), "{line}");
    }

    #[test]
    fn check_prints_no_note_when_its_comparison_failed() {
        let world = World::new();
        assert_eq!(world.piped(&["build"]).code, 0);
        world
            .scratch
            .write(
                "personal/instructions/10-x.md",
                "- be brief, and then some.\n",
            )
            .expect("written");
        let run = world.at_terminal(&["check"], &[]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(run.stderr.contains("error [check]"), "{}", run.stderr);
        assert_eq!(notes(&run.stderr), Vec::<&str>::new());
    }

    #[test]
    fn nothing_is_printed_about_a_cache_miss_when_no_layer_names_an_account() {
        let world = World::new();
        world
            .scratch
            .write(
                "repo/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        let run = world.at_terminal(
            &["build"],
            &[
                ("XDG_CACHE_HOME", "/nonexistent-cache"),
                ("__MISE_DIFF", "x"),
            ],
        );
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(notes(&run.stderr), Vec::<&str>::new());
    }
}
