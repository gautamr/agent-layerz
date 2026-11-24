//! The commit identity step of `alz init`, `alz build`, `alz build --adopt` and `alz check` (spec
//! §9.4, §10.3), through the binary with real git. Every run has its own `HOME`, and
//! `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM` keep git away from the user's own configuration.
//! The global file holds a wrong identity, which no output may print.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_io::EnvSnapshot;
    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const WRONG_NAME: &str = "Shared CI";
    const WRONG_EMAIL: &str = "ci@example.invalid";
    const OWNER_NAME: &str = "Owner Example";
    const OWNER_EMAIL: &str = "owner@client.example";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    impl Run {
        /// No output names the wrong identity of the global config.
        fn never_names_the_wrong_value(&self) {
            for text in [&self.stdout, &self.stderr] {
                assert!(!text.contains(WRONG_NAME), "{text}");
                assert!(!text.contains(WRONG_EMAIL), "{text}");
            }
        }
    }

    /// The fixture of the acceptance suite for commit identities, copied into a scratch folder:
    /// `work/{personal,client,repo}`, with `work/repo` a git repository.
    struct Workspace {
        scratch: ScratchDir,
        work: PathBuf,
    }

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/26-commit-identity")
    }

    fn real_path() -> std::ffi::OsString {
        EnvSnapshot::capture()
            .vars()
            .get("PATH")
            .expect("PATH is set")
            .to_owned()
    }

    impl Workspace {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            copy_tree(&fixture(), &work).expect("the fixture is copied");
            for folder in ["home", "bin"] {
                scratch.make_dir(folder).expect("made");
            }
            scratch
                .write(
                    "home/gitconfig",
                    format!("[user]\n\tname = {WRONG_NAME}\n\temail = {WRONG_EMAIL}\n"),
                )
                .expect("the global config is written");
            let workspace = Self { scratch, work };
            workspace.git("repo", &["init", "--quiet", "--initial-branch=main"]);
            workspace
        }

        fn dir(&self, name: &str) -> PathBuf {
            self.work.join(name)
        }

        fn env(&self) -> Vec<(&'static str, std::ffi::OsString)> {
            let mut path = self.scratch.join("bin").into_os_string();
            path.push(":");
            path.push(real_path());
            vec![
                ("PATH", path),
                ("HOME", self.scratch.join("home").into_os_string()),
                (
                    "XDG_CACHE_HOME",
                    self.scratch.join("cache").into_os_string(),
                ),
                (
                    "GIT_CONFIG_GLOBAL",
                    self.scratch.join("home/gitconfig").into_os_string(),
                ),
                ("GIT_CONFIG_NOSYSTEM", "1".into()),
            ]
        }

        fn git(&self, directory: &str, args: &[&str]) -> String {
            let output = Command::new("git")
                .args(args)
                .current_dir(self.dir(directory))
                .env_clear()
                .envs(self.env())
                .output()
                .expect("git runs");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }

        fn alz(&self, directory: &str, args: &[&str]) -> Run {
            self.alz_with(directory, args, &[])
        }

        fn alz_with(&self, directory: &str, args: &[&str], extra: &[(&str, &str)]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(self.dir(directory))
                .env_clear()
                .envs(self.env())
                .envs(extra.iter().copied())
                .args(args)
                .timeout(Duration::from_secs(60))
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        /// A `git` first on `PATH` that records every call in `bin/calls`, then runs the real one.
        fn record_git_calls(&self) {
            self.stand_in("echo \"$@\" >> \"$CALLS\"");
        }

        /// A stand-in `git` that runs `before`, then the real git with the same arguments.
        fn stand_in(&self, before: &str) {
            let real = Command::new("sh")
                .args(["-c", "command -v git"])
                .env("PATH", real_path())
                .output()
                .expect("sh runs");
            let real = String::from_utf8_lossy(&real.stdout).trim().to_owned();
            let calls = self.scratch.join("bin/calls");
            let script = format!(
                "#!/bin/sh\nREAL='{real}'\nCALLS='{}'\n{before}\nexec \"$REAL\" \"$@\"\n",
                calls.display()
            );
            self.scratch
                .write_executable("bin/git", script)
                .expect("the stand-in is written");
        }

        fn git_was_called(&self) -> bool {
            self.scratch.exists("bin/calls")
        }

        fn local_config(&self) -> String {
            self.scratch
                .read_text("work/repo/.git/config")
                .expect("the local config is read")
        }

        fn without_declarations(&self) {
            for layer in ["personal", "client"] {
                let path = format!("work/{layer}/layer.yaml");
                let text = self.scratch.read_text(&path).expect("read");
                let kept = text
                    .split("commit_identity:")
                    .next()
                    .expect("a first part")
                    .to_owned();
                self.scratch.write(&path, kept).expect("written");
            }
        }

        fn fresh_repository(&self, name: &str) {
            self.scratch.make_dir(format!("work/{name}")).expect("made");
            self.git(name, &["init", "--quiet", "--initial-branch=main"]);
        }
    }

    // --- build --------------------------------------------------------------------------------

    #[test]
    fn build_warns_about_a_wrong_identity_by_scope_and_file_and_writes_nothing() {
        let workspace = Workspace::new();
        let before = workspace.local_config();
        let run = workspace.alz("repo", &["build"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr.contains(
                "warning [identity] git takes the name (user.name) from global config (~/gitconfig)"
            ),
            "{}",
            run.stderr
        );
        assert!(
            run.stderr
                .contains("warning [identity] git takes the email (user.email)")
        );
        assert!(
            run.stderr
                .contains("declared by layer 'client' (path:../client)")
        );
        assert!(run.stderr.contains(&format!(
            "fix: git config --local user.email '{OWNER_EMAIL}'"
        )));
        assert!(!run.stdout.contains("set user."), "{}", run.stdout);
        assert_eq!(workspace.local_config(), before);
        run.never_names_the_wrong_value();
    }

    #[test]
    fn build_adopt_adds_the_declared_identity_to_the_local_config_and_leaves_the_global_one() {
        let workspace = Workspace::new();
        let run = workspace.alz("repo", &["build", "--adopt"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stdout.contains(&format!(
            "set user.name in .git/config to '{OWNER_NAME}' (declared by layer 'client' (path:../client))"
        )));
        assert!(run.stdout.contains("set user.email in .git/config to"));
        assert!(!run.stderr.contains("[identity]"), "{}", run.stderr);
        assert_eq!(
            workspace.git("repo", &["config", "--local", "user.name"]),
            OWNER_NAME
        );
        assert_eq!(
            workspace.git("repo", &["config", "--local", "user.email"]),
            OWNER_EMAIL
        );
        let global = workspace
            .scratch
            .read_text("home/gitconfig")
            .expect("the global config is read");
        assert!(global.contains(WRONG_NAME), "the global file is untouched");
        let check = workspace.alz("repo", &["check"]);
        assert_eq!(check.code, 0, "{}", check.stderr);
        run.never_names_the_wrong_value();
    }

    #[test]
    fn build_adopt_never_replaces_a_different_local_value_and_check_fails_for_it() {
        let workspace = Workspace::new();
        workspace.git(
            "repo",
            &["config", "--local", "user.email", "other@example.com"],
        );
        let run = workspace.alz("repo", &["build", "--adopt"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stderr.contains(
            "warning [identity] user.email in .git/config differs from the identity declared by layer 'client'"
        ));
        assert!(run.stderr.contains("alz never replaces it"));
        assert_eq!(
            workspace.git("repo", &["config", "--local", "user.email"]),
            "other@example.com"
        );
        assert!(!run.stderr.contains("other@example.com"), "{}", run.stderr);
        let check = workspace.alz("repo", &["check"]);
        assert_eq!(check.code, 1);
        assert!(check.stderr.contains(
            "error [identity] git takes the email (user.email) from local config (.git/config)"
        ));
        assert!(!check.stderr.contains("other@example.com"));
    }

    #[test]
    fn an_environment_variable_is_named_and_its_value_is_not() {
        let workspace = Workspace::new();
        let run = workspace.alz_with(
            "repo",
            &["build", "--adopt"],
            &[("GIT_COMMITTER_NAME", "Env Person")],
        );
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stderr.contains("the environment (GIT_COMMITTER_NAME)"));
        assert!(run.stderr.contains("fix: unset GIT_COMMITTER_NAME"));
        assert!(!run.stderr.contains("Env Person"));
        assert!(!run.stdout.contains("Env Person"));
    }

    #[test]
    fn a_directory_that_is_not_a_git_repository_gets_a_note_and_check_passes() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .remove_tree("work/repo/.git")
            .expect("the repository is removed");
        let build = workspace.alz("repo", &["build"]);
        assert_eq!(build.code, 0, "{}", build.stderr);
        assert!(build.stderr.contains(
            "note [identity] this is not a git repository, so the commit identity was not checked"
        ));
        let check = workspace.alz("repo", &["check"]);
        assert_eq!(check.code, 0, "{}", check.stderr);
        assert!(
            check
                .stderr
                .contains("note [identity] this is not a git repository")
        );
    }

    #[test]
    fn a_link_at_the_config_file_is_not_written_through_and_is_a_warning() {
        let workspace = Workspace::new();
        let target = workspace.scratch.join("home/elsewhere-config");
        workspace
            .scratch
            .write("home/elsewhere-config", workspace.local_config())
            .expect("written");
        workspace
            .scratch
            .remove_file("work/repo/.git/config")
            .expect("removed");
        workspace
            .scratch
            .make_link("work/repo/.git/config", &target)
            .expect("linked");
        let before = workspace
            .scratch
            .read_text("home/elsewhere-config")
            .expect("read");
        let run = workspace.alz("repo", &["build", "--adopt"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stderr.contains(
            "warning [identity] .git/config is not a regular file, so alz did not write the git identity declared by layer 'client'"
        ));
        assert!(
            run.stderr
                .contains("Run by hand: git config --local user.name")
        );
        assert!(!run.stdout.contains("set user."));
        assert_eq!(
            workspace
                .scratch
                .read_text("home/elsewhere-config")
                .expect("read"),
            before
        );
        run.never_names_the_wrong_value();
    }

    #[test]
    fn a_link_put_at_the_config_file_during_the_write_is_an_error_and_exit_1() {
        let workspace = Workspace::new();
        let target = workspace.scratch.join("home/elsewhere-config");
        workspace
            .scratch
            .write("home/elsewhere-config", "[core]\n\tbare = false\n")
            .expect("written");
        workspace.stand_in(&format!(
            "if [ \"$1\" = config ] && [ \"$2\" = --file ] && [ \"$4\" = --end-of-options ]; then\n  rm -f \"$3\"; ln -s '{}' \"$3\"\nfi",
            target.display()
        ));
        let run = workspace.alz("repo", &["build", "--adopt"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(run.stderr.contains(
            "error [identity] .git/config is not the regular file alz checked before git wrote user.name"
        ));
        assert!(run.stderr.contains("check .git/config by hand"));
        assert!(
            !run.stdout.contains("set user.email"),
            "nothing more is written"
        );
        assert!(
            !workspace
                .scratch
                .read_text("home/elsewhere-config")
                .expect("read")
                .contains(OWNER_EMAIL)
        );
        run.never_names_the_wrong_value();
    }

    // --- check --------------------------------------------------------------------------------

    #[test]
    fn check_fails_on_a_wrong_identity_with_the_last_line_that_says_so() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("repo", &["build"]).code, 0);
        let run = workspace.alz("repo", &["check"]);
        assert_eq!(run.code, 1);
        assert!(
            run.stderr
                .contains("error [identity] git takes the name (user.name) from global config")
        );
        assert_eq!(
            run.stderr.lines().last(),
            Some("check failed: the git identity differs from the declared commit identity")
        );
        assert!(!run.stdout.contains("check passed"));
        run.never_names_the_wrong_value();
    }

    #[test]
    fn check_reports_a_failed_comparison_and_the_identity_and_ends_with_the_comparison() {
        let workspace = Workspace::new();
        let run = workspace.alz("repo", &["check"]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.contains("error [check]"));
        assert!(run.stderr.contains("error [identity]"));
        assert_eq!(
            run.stderr.lines().last(),
            Some("check failed: run `alz build` and commit the result")
        );
    }

    #[test]
    fn skip_identity_passes_and_starts_no_git() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("repo", &["build"]).code, 0);
        workspace.record_git_calls();
        let run = workspace.alz("repo", &["check", "--skip-identity"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(!run.stderr.contains("[identity]"), "{}", run.stderr);
        assert!(!workspace.git_was_called(), "no git command ran");
    }

    #[test]
    fn check_fails_when_git_cannot_be_used() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("repo", &["build"]).code, 0);
        workspace.stand_in("exit 99");
        let run = workspace.alz("repo", &["check"]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.contains(
            "error [identity] git is not available or failed, so the commit identity could not be checked"
        ));
        let build = workspace.alz("repo", &["build"]);
        assert_eq!(build.code, 0, "{}", build.stderr);
        assert!(
            build
                .stderr
                .contains("warning [identity] git is not available or failed")
        );
    }

    // --- init ---------------------------------------------------------------------------------

    #[test]
    fn init_sets_the_identity_of_the_first_layer_in_a_fresh_repository() {
        let workspace = Workspace::new();
        workspace.fresh_repository("fresh");
        let run = workspace.alz("fresh", &["init", "--stack", "../personal"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stdout.contains(
            "set user.email in .git/config to 'owner@example.com' (declared by layer 'personal' (path:../personal))"
        ));
        assert_eq!(
            workspace.git("fresh", &["config", "--local", "user.name"]),
            OWNER_NAME
        );
        let created = run.stdout.find("created layer.yaml").expect("scaffolded");
        let set = run.stdout.find("set user.name").expect("set");
        assert!(created < set, "the step follows the scaffold");
        run.never_names_the_wrong_value();
    }

    #[test]
    fn init_only_notes_that_build_adopt_will_set_it_when_the_stack_cannot_be_resolved_yet() {
        let workspace = Workspace::new();
        workspace.fresh_repository("fresh");
        let run = workspace.alz("fresh", &["init", "--stack", "../missing"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stderr.contains(
            "note [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it"
        ));
        assert!(!run.stderr.contains("warning"), "{}", run.stderr);
        assert!(
            !workspace
                .scratch
                .read_text("work/fresh/.git/config")
                .expect("read")
                .contains("user")
        );
    }

    #[test]
    fn init_warns_about_a_layer_that_may_not_declare_writes_nothing_and_exits_0() {
        let workspace = Workspace::new();
        workspace.scratch.make_dir("work/group").expect("made");
        workspace
            .scratch
            .write(
                "work/group/layer.yaml",
                format!("name: group\nkind: group\nvisibility: private\ncommit_identity:\n  name: {OWNER_NAME}\n  email: {OWNER_EMAIL}\n"),
            )
            .expect("written");
        workspace.fresh_repository("fresh");
        let run = workspace.alz(
            "fresh",
            &["init", "--stack", "../personal", "--stack", "../group"],
        );
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr.contains("warning [identity] group:layer.yaml"),
            "{}",
            run.stderr
        );
        assert!(!run.stdout.contains("set user."));
        assert!(
            !workspace
                .scratch
                .read_text("work/fresh/.git/config")
                .expect("read")
                .contains("user")
        );
        let build = workspace.alz("fresh", &["build"]);
        assert_eq!(build.code, 1, "build fails on the same problem");
    }

    // --- no declaration -----------------------------------------------------------------------

    #[test]
    fn without_a_declared_identity_init_build_and_check_run_no_git_and_print_no_identity_line() {
        let workspace = Workspace::new();
        workspace.without_declarations();
        workspace.fresh_repository("fresh");
        workspace.record_git_calls();
        for (directory, args) in [
            ("fresh", vec!["init", "--stack", "../personal"]),
            ("fresh", vec!["build"]),
            ("fresh", vec!["check"]),
            ("repo", vec!["build", "--adopt"]),
            ("repo", vec!["check"]),
        ] {
            let run = workspace.alz(directory, &args);
            assert_eq!(run.code, 0, "{args:?}: {}", run.stderr);
            assert!(
                !run.stderr.contains("[identity]"),
                "{args:?}: {}",
                run.stderr
            );
            assert!(
                !run.stdout.contains("set user."),
                "{args:?}: {}",
                run.stdout
            );
        }
        assert!(!workspace.git_was_called(), "no git command ran");
    }
}
