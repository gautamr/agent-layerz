//! Where the account note of `alz build` and `alz check` is never printed (spec §10.3 "The
//! account note", §7.13 "Trust"): for a root at or above `$HOME`, inside an account folder, and
//! inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names. A control run in the usual place
//! prints the note, so each silence is the place's doing. Runs go through a pseudo-terminal
//! (`pty-run`), and every run has its own home and `XDG_*` folders.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, pty_run_executable};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const NOTE_START: &str = "note [account-allow] ";

    struct Run {
        stderr: String,
    }

    /// A repository with a personal layer, a client layer that names an account and its own
    /// layer, all below `base` of the scratch folder; `home` is the folder `HOME` names.
    struct World {
        scratch: ScratchDir,
        base: &'static str,
        home: &'static str,
    }

    impl World {
        fn new(base: &'static str, home: &'static str) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            for folder in ["home", "config", "state", "cache"] {
                scratch.make_dirs(folder).expect("made");
            }
            let world = Self {
                scratch,
                base,
                home,
            };
            for (path, text) in [
                (
                    "personal/layer.yaml",
                    "name: me\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
                ),
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
                let path = Path::new(base).join(path);
                if let Some(parent) = path.parent() {
                    world.scratch.make_dirs(parent).expect("made");
                }
                world.scratch.write(path, text).expect("written");
            }
            world
        }

        fn command(&self, program: &Path, environment: &[(&str, String)]) -> Command {
            let mut command = Command::new(program);
            command
                .current_dir(self.scratch.join(self.base).join("repo"))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join(self.home))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .envs(environment.iter().map(|(name, value)| (name, value)))
                .timeout(Duration::from_secs(60));
            command
        }

        fn finish(mut command: Command) -> Run {
            let output = command.output().expect("alz runs and ends in time");
            Run {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        /// `alz <argument>` with standard input a pipe: nobody is at a terminal.
        fn piped(&self, argument: &str, environment: &[(&str, String)]) -> Run {
            let program = cargo_bin_cmd!("alz");
            let mut command = self.command(program.get_program().as_ref(), environment);
            command.arg(argument);
            Self::finish(command)
        }

        /// `alz <argument>` on a pseudo-terminal.
        fn at_terminal(&self, argument: &str, environment: &[(&str, String)]) -> Run {
            let program = cargo_bin_cmd!("alz");
            let driver = pty_run_executable().expect("pty-run is built with the workspace");
            let mut command = self.command(&driver, environment);
            command.arg("--").arg(program.get_program()).arg(argument);
            Self::finish(command)
        }

        fn path(&self, relative: &str) -> String {
            self.scratch
                .real_path()
                .expect("a real path")
                .join(relative)
                .display()
                .to_string()
        }

        /// Moves the whole tree (the repository and its layers) to `to`.
        fn relocate(&mut self, to: &'static str) {
            self.scratch
                .make_dirs(Path::new(to).parent().expect("a parent"))
                .expect("made");
            self.scratch.rename(self.base, to).expect("moved");
            self.base = to;
        }

        /// The lines of standard error that start with the note, from `build` and from `check`
        /// at a terminal. In these places both commands refuse with the error `write` as well.
        fn notes(&self, environment: &[(&str, String)]) -> Vec<String> {
            ["build", "check"]
                .into_iter()
                .flat_map(|command| {
                    let run = self.at_terminal(command, environment);
                    run.stderr
                        .lines()
                        .filter(|line| line.starts_with(NOTE_START))
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .collect()
        }

        /// A world built where the note is printed, so that `layer.lock` is current.
        fn built(base: &'static str) -> Self {
            let world = Self::new(base, "home");
            let built = world.piped("build", &[]);
            assert_eq!(built.stderr, "", "the build is silent");
            world
        }
    }

    /// The control: in the usual place the note is printed once by `build` and once by `check`.
    #[test]
    fn the_note_is_printed_in_a_repository_below_the_home_folder() {
        let world = World::built("work");
        assert_eq!(world.notes(&[]).len(), 2);
    }

    /// Spec §10.3 "The account note": never for a root at `$HOME`, nor above it.
    #[test]
    fn no_note_is_printed_for_a_root_at_or_above_home() {
        let mut at = World::built("work");
        at.home = "work/repo";
        assert_eq!(at.notes(&[]), Vec::<String>::new(), "at home");
        let mut above = World::built("work");
        above.scratch.make_dirs("work/repo/h").expect("made");
        above.home = "work/repo/h";
        assert_eq!(above.notes(&[]), Vec::<String>::new(), "above home");
    }

    /// Spec §10.3 "The account note": never for a root inside an account folder.
    #[test]
    fn no_note_is_printed_inside_an_account_folder() {
        for folder in ["home/.claude-work", "home/.codex-work"] {
            let mut world = World::built("work");
            world.relocate(folder);
            assert_eq!(world.notes(&[]), Vec::<String>::new(), "{folder}");
        }
    }

    /// Spec §10.3 "The account note": never for a root inside a folder `CLAUDE_CONFIG_DIR` or
    /// `CODEX_HOME` names.
    #[test]
    fn no_note_is_printed_inside_a_folder_the_configuration_variables_name() {
        for variable in ["CLAUDE_CONFIG_DIR", "CODEX_HOME"] {
            let mut world = World::built("work");
            world.relocate("configured");
            let folder = world.path("configured");
            assert_eq!(
                world.notes(&[(variable, folder)]),
                Vec::<String>::new(),
                "{variable}"
            );
        }
    }
}
