//! `alz doctor` and `alz ls --live` (spec §10.3, TASK-55.6.11) against a fake `claude` script.
//! Every run has its own `HOME`, so nothing reads the real `~/.claude*`. The deadline of the
//! runner is covered where its length can be set, in the `alz-io` tests.

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

    /// Prints what `claude --version` prints.
    fn version_script(text: &str) -> String {
        format!("#!/bin/sh\nprintf '%s\\n' '{text}'\n")
    }

    const EXPECTED_ARGUMENTS: &str =
        "-p Reply with OK. --output-format stream-json --verbose --max-turns 1";

    /// A session that checks it was asked as the spec says, then prints `init` and `stderr`.
    fn session_script(init: &str, stderr_text: &str) -> String {
        format!(
            "#!/bin/sh\n[ \"$*\" = \"{EXPECTED_ARGUMENTS}\" ] || exit 3\n[ -z \"$GITHUB_TOKEN\" ] || exit 4\nprintf '%s\\n' '{stderr_text}' >&2\nprintf '%s\\n' '{init}'\n"
        )
    }

    impl Workspace {
        fn of(fixture: &str, repository: &str) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../test/fixtures")
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

        fn claude(&self, script: &str) -> PathBuf {
            self.scratch
                .write_executable("fake-claude", script)
                .expect("written");
            self.scratch.join("fake-claude")
        }

        fn alz(&self, arguments: &[&str], environment: &[(&str, &Path)]) -> Run {
            self.alz_with(arguments, environment, &[])
        }

        /// As `alz`, with more variables whose values are text.
        fn alz_with(
            &self,
            arguments: &[&str],
            environment: &[(&str, &Path)],
            text: &[(&str, &str)],
        ) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("ALZ_CLAUDE_BIN", self.scratch.join("no-claude-here"))
                .env("GITHUB_TOKEN", "must-not-reach-claude")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(arguments);
            for (name, value) in environment {
                alz.env(name, value);
            }
            for (name, value) in text {
                alz.env(name, value);
            }
            let output = alz.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn doctor_with(&self, claude: &Path) -> Run {
            self.alz(&["doctor"], &[("ALZ_CLAUDE_BIN", claude)])
        }
    }

    fn live_workspace() -> Workspace {
        Workspace::of("22-ls", "repo")
    }

    #[test]
    fn doctor_reports_a_user_skill_with_a_built_name_as_a_collision_and_fails() {
        let workspace = Workspace::of("05-skill-override", "repo");
        let account = workspace.scratch.join("work/account");
        let run = workspace.alz(&["doctor"], &[("CLAUDE_CONFIG_DIR", &account)]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stdout.contains(&format!(
                "  {}: 1 collision\n    collision: skill 'stack-conventions' also exists in {}/skills (Claude: a user skill beats the project one, silently)\n",
                account.display(),
                account.display()
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

    #[test]
    fn doctor_gives_the_same_report_under_two_empty_account_folders() {
        let reports: Vec<String> = ["account-a", "account-b"]
            .iter()
            .map(|name| {
                let workspace = Workspace::of("01-determinism", "repo");
                workspace.scratch.make_dir(name).expect("made");
                let account = workspace.scratch.join(name);
                let run = workspace.alz(&["doctor"], &[("CLAUDE_CONFIG_DIR", &account)]);
                assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
                assert!(run.stdout.contains("no collisions"));
                run.stdout
                    .replace(workspace.scratch.path().to_string_lossy().as_ref(), "<s>")
                    .replace(name, "<account>")
            })
            .collect();
        assert_eq!(reports.first(), reports.get(1));
    }

    #[test]
    fn doctor_warns_about_a_claude_md_at_the_client_root() {
        let workspace = Workspace::of("16-hub", "repo-a");
        let root = workspace
            .scratch
            .real_path()
            .expect("real path")
            .join("work");
        // The hub here is not a git repository, so the build has an error; the report still names the
        // client root, because the manifest was read.
        let run = workspace.alz(&["doctor"], &[]);
        assert!(
            run.stdout.contains(&format!(
                "warning: client root {}/CLAUDE.md exists; remove it (put client content in the hub layer)",
                root.display()
            )),
            "{}{}",
            run.stdout,
            run.stderr
        );
        assert!(!run.stdout.contains("is a git repo"), "{}", run.stdout);
    }

    #[test]
    fn doctor_prints_the_version_a_fake_claude_gives_and_whether_it_is_verified() {
        let workspace = live_workspace();
        let verified = workspace.claude(&version_script("2.1.311 (Claude Code)"));
        let run = workspace.doctor_with(&verified);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .contains("claude: 2.1.311 (verified range >=2.1.311 <2.2.0)\n"),
            "{}",
            run.stdout
        );
        let newer = workspace.claude(&version_script("2.2.0 (Claude Code)"));
        let run = workspace.doctor_with(&newer);
        assert_eq!(run.code, 0, "a version outside the range is a warning");
        assert!(run.stdout.contains(
            "warning: claude 2.2.0 is outside the verified range >=2.1.311 <2.2.0; re-check docs/target-facts.md\n"
        ));
    }

    #[test]
    fn doctor_skips_the_version_check_when_claude_is_missing_or_prints_no_version() {
        let workspace = live_workspace();
        let missing = workspace.scratch.join("absent");
        let run = workspace.doctor_with(&missing);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .contains("claude: not found; version check skipped\n")
        );
        let silent = workspace.claude("#!/bin/sh\necho no version here\n");
        assert!(
            workspace
                .doctor_with(&silent)
                .stdout
                .contains("claude: not found; version check skipped\n")
        );
        let failing = workspace.claude("#!/bin/sh\necho 2.1.400\nexit 1\n");
        assert!(
            workspace
                .doctor_with(&failing)
                .stdout
                .contains("claude: not found; version check skipped\n")
        );
    }

    #[test]
    fn doctor_never_prints_an_escape_sequence_from_claude_or_the_file_system() {
        let workspace = live_workspace();
        let hostile = workspace
            .claude("#!/bin/sh\nprintf '\\033[2J\\033]0;owned\\007 2.1.400 \\033[31m\\n'\n");
        workspace
            .scratch
            .make_dir("acct\u{1b}[2J\u{202e}x")
            .expect("made");
        let folder = workspace.scratch.join("acct\u{1b}[2J\u{202e}x");
        let run = workspace.alz(
            &["doctor"],
            &[("ALZ_CLAUDE_BIN", &hostile), ("CLAUDE_CONFIG_DIR", &folder)],
        );
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout.contains("claude: 2.1.400 (verified range"),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains('\u{1b}') && !run.stdout.contains('\u{202e}'));
        assert!(
            run.stdout.contains("acct[2Jx: no collisions"),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn doctor_without_a_manifest_reports_the_error_as_a_problem() {
        let workspace = live_workspace();
        let run = workspace.alz(
            &["doctor"],
            &[("ALZ_CLAUDE_BIN", &workspace.scratch.join("x"))],
        );
        assert_eq!(run.code, 0, "{}", run.stdout);
        let bare = workspace.scratch.join("bare");
        workspace.scratch.make_dir("bare").expect("made");
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(&bare)
            .env("HOME", workspace.scratch.join("home"))
            .env_remove("CLAUDE_CONFIG_DIR")
            .arg("doctor")
            .output()
            .expect("runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{stdout}");
        assert!(stdout.contains("error [no-manifest]"), "{stdout}");
        assert!(stdout.ends_with("doctor: 1 problem\n"), "{stdout}");
    }

    #[test]
    fn ls_live_passes_when_claude_loaded_everything_and_shows_its_warnings() {
        let workspace = live_workspace();
        let claude = workspace.claude(&session_script(
            r#"{"type":"system","subtype":"init","agents":["architect","Explore"],"skills":["review"],"mcp_servers":[]}"#,
            "Ignoring 1 permissions.additionalDirectories entry",
        ));
        let run = workspace.alz(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &claude)]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(run.stdout.contains(&format!(
            "\nasking {} what it loads here (one short model call)…\n",
            claude.display()
        )));
        assert!(
            run.stdout
                .contains("claude loaded 2 agents, 1 skills and commands\n")
        );
        assert!(run.stdout.contains("  not from layers, agents: Explore\n"));
        assert!(
            run.stdout
                .ends_with("ls --live: Claude matches the build\n")
        );
        assert!(
            run.stderr
                .contains("warning [claude] Ignoring 1 permissions.additionalDirectories entry\n"),
            "{}",
            run.stderr
        );
    }

    const INIT_EVENT: &str = r#"{"type":"system","subtype":"init","agents":["architect","Explore"],"skills":["review"],"mcp_servers":[]}"#;

    /// The names, never the values, of the variables the probe's child saw that belong to a
    /// cloud provider or to the planted `GITHUB_TOKEN`, sorted, one per line.
    fn cloud_names_the_probe_saw(switches: &[(&str, &str)]) -> String {
        let workspace = live_workspace();
        let seen = workspace.scratch.join("seen-names");
        let claude = workspace.claude(&format!(
            "#!/bin/sh\nenv | sed 's/=.*//' | grep -E '^(AWS_|GOOGLE_|CLOUD_ML_|ANTHROPIC_VERTEX_|CLAUDE_CODE_USE_|GITHUB_)' | sort > '{}'\nprintf '%s\\n' '{INIT_EVENT}'\n",
            seen.display()
        ));
        let planted = [
            ("AWS_PROFILE", "planted"),
            ("GOOGLE_APPLICATION_CREDENTIALS", "planted"),
            ("CLOUD_ML_REGION", "planted"),
            ("ANTHROPIC_VERTEX_PROJECT_ID", "planted"),
        ];
        let text: Vec<(&str, &str)> = planted.iter().chain(switches).copied().collect();
        let run = workspace.alz_with(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &claude)], &text);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        workspace
            .scratch
            .read_text("seen-names")
            .expect("the fake claude wrote the names it saw")
    }

    #[test]
    fn ls_live_hands_claude_a_providers_variables_only_when_its_switch_is_on() {
        assert_eq!(cloud_names_the_probe_saw(&[]), "");
        assert_eq!(
            cloud_names_the_probe_saw(&[("CLAUDE_CODE_USE_BEDROCK", "1")]),
            "AWS_PROFILE\nCLAUDE_CODE_USE_BEDROCK\n"
        );
        assert_eq!(
            cloud_names_the_probe_saw(&[("CLAUDE_CODE_USE_VERTEX", "1")]),
            "ANTHROPIC_VERTEX_PROJECT_ID\nCLAUDE_CODE_USE_VERTEX\nCLOUD_ML_REGION\nGOOGLE_APPLICATION_CREDENTIALS\n"
        );
        assert_eq!(
            cloud_names_the_probe_saw(&[
                ("CLAUDE_CODE_USE_BEDROCK", "1"),
                ("CLAUDE_CODE_USE_VERTEX", "1")
            ]),
            "ANTHROPIC_VERTEX_PROJECT_ID\nAWS_PROFILE\nCLAUDE_CODE_USE_BEDROCK\nCLAUDE_CODE_USE_VERTEX\nCLOUD_ML_REGION\nGOOGLE_APPLICATION_CREDENTIALS\n"
        );
    }

    #[test]
    fn ls_live_fails_naming_the_agent_claude_did_not_load() {
        let workspace = live_workspace();
        let claude = workspace.claude(&session_script(
            r#"{"type":"system","subtype":"init","agents":["Explore"],"skills":[],"mcp_servers":[]}"#,
            "",
        ));
        let run = workspace.alz(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &claude)]);
        assert_eq!(run.code, 1);
        assert!(
            run.stderr
                .contains("error [ls] agent 'architect' was built but Claude did not load it\n")
        );
        assert!(
            run.stdout
                .ends_with("ls --live: Claude does not match the build\n")
        );
    }

    #[test]
    fn ls_live_is_an_error_when_claude_is_missing_or_prints_no_init_event() {
        let workspace = live_workspace();
        let missing = workspace.scratch.join("absent");
        let run = workspace.alz(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &missing)]);
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains(&format!(
                "error [ls] could not run {}: not found\n",
                missing.display()
            )),
            "{}",
            run.stderr
        );
        let silent = workspace.claude("#!/bin/sh\necho 'Not logged in' >&2\nexit 1\n");
        let run = workspace.alz(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &silent)]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.contains(&format!(
            "error [ls] {} printed no init event: Not logged in\n",
            silent.display()
        )));
    }

    #[test]
    fn ls_live_never_prints_an_escape_sequence_from_claude() {
        let workspace = live_workspace();
        let claude = workspace.claude(&session_script(
            r#"{"type":"system","subtype":"init","agents":["architect","Evil\u001b[2J\u202e"],"skills":["a\u001b]0;x\u0007b"],"mcp_servers":[]}"#,
            "\u{1b}[31mred\u{1b}[0m \u{7}bell",
        ));
        let run = workspace.alz(&["ls", "--live"], &[("ALZ_CLAUDE_BIN", &claude)]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        for text in [&run.stdout, &run.stderr] {
            assert!(
                !text.contains('\u{1b}') && !text.contains('\u{7}') && !text.contains('\u{202e}')
            );
        }
        assert!(
            run.stdout.contains("not from layers, agents: Evil[2J\n"),
            "{}",
            run.stdout
        );
        assert!(
            run.stderr.contains("warning [claude] [31mred[0m bell\n"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn ls_without_live_does_not_start_claude() {
        let workspace = live_workspace();
        let claude = workspace.claude("#!/bin/sh\ntouch started\n");
        let run = workspace.alz(&["ls"], &[("ALZ_CLAUDE_BIN", &claude)]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(!workspace.scratch.exists("work/repo/started"));
    }
}
