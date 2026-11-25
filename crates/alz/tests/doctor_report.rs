//! The lines of `alz doctor` that the first tests of `doctor_live.rs` leave out (spec §10.3
//! "`alz doctor`", TASK-55.9): the agent collision, the note about an account's `CLAUDE.md`, the
//! warnings for the folders above the repository and for a client root that is a git repository,
//! and the `build: <e> errors, <w> warnings` line with its indented diagnostics. Every run has its
//! own `HOME`, so nothing reads the real `~/.claude*`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn of(fixture: &str, repository: &str) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures")
                    .join(fixture),
                &work,
            )
            .expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            Self {
                repository: work.join(repository),
                scratch,
            }
        }

        fn doctor(&self, environment: &[(&str, &Path)]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("ALZ_CLAUDE_BIN", self.scratch.join("no-claude-here"))
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .arg("doctor");
            for (name, value) in environment {
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

    /// Spec §10.3 `alz doctor`: an account folder that holds an agent with a built name gets the
    /// collision line for agents, and its `CLAUDE.md` the note, both under the folder's line.
    #[test]
    fn doctor_reports_an_account_agent_with_a_built_name_and_the_account_claude_md() {
        let workspace = Workspace::of("22-ls", "repo");
        workspace
            .scratch
            .make_dirs("work/account/agents")
            .expect("made");
        workspace
            .scratch
            .write("work/account/agents/architect.md", "mine\n")
            .expect("written");
        workspace
            .scratch
            .write("work/account/CLAUDE.md", "mine\n")
            .expect("written");
        let account = workspace.scratch.join("work/account");
        let run = workspace.doctor(&[("CLAUDE_CONFIG_DIR", &account)]);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        let folder = account.display();
        assert!(
            run.stdout.contains(&format!(
                "  {folder}: 1 collision\n    collision: agent 'architect' also exists in {folder}/agents (move it into a layer)\n    note: {folder}/CLAUDE.md is loaded into every session as well\n"
            )),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.ends_with("doctor: 1 problem\n"),
            "{}",
            run.stdout
        );
        assert_eq!(run.stderr, "");
    }

    /// Spec §10.3 `alz doctor`: the `CLAUDE.md` note appears for an account folder without
    /// collisions too, below its `no collisions` line.
    #[test]
    fn doctor_notes_the_account_claude_md_below_the_no_collisions_line() {
        let workspace = Workspace::of("22-ls", "repo");
        workspace.scratch.make_dir("work/account").expect("made");
        workspace
            .scratch
            .write("work/account/CLAUDE.md", "mine\n")
            .expect("written");
        let account = workspace.scratch.join("work/account");
        let run = workspace.doctor(&[("CLAUDE_CONFIG_DIR", &account)]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        let folder = account.display();
        assert!(
            run.stdout.contains(&format!(
                "  {folder}: no collisions\n    note: {folder}/CLAUDE.md is loaded into every session as well\n"
            )),
            "{}",
            run.stdout
        );
    }

    /// Spec §10.3 `alz doctor`: each folder above the repository warns when it holds a
    /// `CLAUDE.md`, and when it is a git repository with `.claude/`; neither changes the exit code.
    #[test]
    fn doctor_warns_about_the_folders_above_the_repository() {
        let workspace = Workspace::of("22-ls", "repo");
        workspace
            .scratch
            .write("work/CLAUDE.md", "above\n")
            .expect("written");
        workspace.scratch.make_dirs(".git").expect("made");
        workspace.scratch.make_dirs(".claude").expect("made");
        let real = workspace.scratch.real_path().expect("real path");
        let run = workspace.doctor(&[]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        let above = format!(
            "warning: {work}/CLAUDE.md is loaded too (ancestor directory)\n",
            work = real.join("work").display()
        );
        let git = format!(
            "warning: {} is a git repo with .claude/; its skills or agents may load in this repo\n",
            real.display()
        );
        assert!(run.stdout.contains(&above), "{}", run.stdout);
        assert!(run.stdout.contains(&git), "{}", run.stdout);
        assert!(
            run.stdout.find(&above) < run.stdout.find(&git),
            "nearest folder first: {}",
            run.stdout
        );
        assert!(run.stdout.ends_with("doctor: ok\n"), "{}", run.stdout);
    }

    /// Spec §10.3 `alz doctor`: a client root that is a git repository is warned about, and it is
    /// not also listed as an ancestor ("each folder above the repository except the client root").
    #[test]
    fn doctor_warns_that_the_client_root_is_a_git_repository_and_lists_it_once() {
        let workspace = Workspace::of("16-hub", "repo-a");
        workspace.scratch.make_dirs("work/.git").expect("made");
        workspace.scratch.make_dirs("work/.claude").expect("made");
        let root = workspace
            .scratch
            .real_path()
            .expect("real path")
            .join("work");
        let run = workspace.doctor(&[]);
        let warning = format!(
            "warning: client root {} is a git repo; it should not be (its .claude/ and AGENTS.md would load in every repo)\n",
            root.display()
        );
        assert!(
            run.stdout.contains(&warning),
            "{}{}",
            run.stdout,
            run.stderr
        );
        assert!(
            !run.stdout.contains("is a git repo with .claude/"),
            "the client root is not an ancestor: {}",
            run.stdout
        );
        assert!(
            !run.stdout.contains(&format!(
                "warning: {}/CLAUDE.md is loaded too",
                root.display()
            )),
            "{}",
            run.stdout
        );
    }

    /// Spec §10.3 `alz doctor`: the build's errors and warnings follow `build: <e> errors,
    /// <w> warnings`, each indented by two spaces; an error is a problem and sets exit 1.
    #[test]
    fn doctor_prints_the_build_line_with_its_diagnostics_indented() {
        let workspace = Workspace::of("04-tool-narrowing", "repo-widen");
        let run = workspace.doctor(&[]);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        let after_build = run
            .stdout
            .split_once("\nbuild: ")
            .map(|(_, rest)| rest)
            .expect("a build line");
        let mut lines = after_build.lines();
        assert_eq!(lines.next(), Some("1 error, 0 warnings"), "{}", run.stdout);
        let diagnostic = lines.next().expect("the diagnostic");
        assert!(
            diagnostic.starts_with("  error [tool-widen] "),
            "{}",
            run.stdout
        );
        assert_eq!(lines.next(), Some("doctor: 1 problem"), "{}", run.stdout);
        assert_eq!(lines.next(), None);
        assert_eq!(run.stderr, "");
    }

    /// Spec §10.3 `alz doctor`: a warning of the build is listed and counted, and is not a problem.
    #[test]
    fn doctor_lists_a_build_warning_and_stays_ok() {
        let workspace = Workspace::of("06-slots", "repo-loose");
        let run = workspace.doctor(&[]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .contains("\nbuild: 0 errors, 1 warning\n  warning [slot-missing] "),
            "{}",
            run.stdout
        );
        assert!(run.stdout.ends_with("doctor: ok\n"), "{}", run.stdout);
    }

    /// Spec §10.3 `alz doctor`: when the build has an error no built names are known, so an
    /// account folder that holds an agent of the same name as one of the stack finds no collision.
    #[test]
    fn doctor_finds_no_collision_when_the_build_has_an_error() {
        let workspace = Workspace::of("04-tool-narrowing", "repo-widen");
        workspace
            .scratch
            .make_dirs("work/account/agents")
            .expect("made");
        workspace
            .scratch
            .write("work/account/agents/builder.md", "mine\n")
            .expect("written");
        let account = workspace.scratch.join("work/account");
        let run = workspace.doctor(&[("CLAUDE_CONFIG_DIR", &account)]);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .contains(&format!("  {}: no collisions\n", account.display())),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("collision: "), "{}", run.stdout);
        assert!(
            run.stdout.ends_with("doctor: 1 problem\n"),
            "{}",
            run.stdout
        );
    }
}
