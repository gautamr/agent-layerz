//! `alz explain` for hooks and servers, and the text it prints (spec §10.3 "`alz explain <type>
//! <name>`", TASK-55.9): the `hook <id>` and `server <id>` actions, and the control and format
//! characters stripped from the merged text with line feeds and tabs kept.

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

        fn explain(&self, arguments: &[&str]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .arg("explain")
                .args(arguments)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }
    }

    /// Spec §10.3 `alz explain`: `hook <id>` is the action of a hook's contribution, whose source
    /// is the file that holds it, and the merged result is `.claude/settings.json`.
    #[test]
    fn explain_hook_names_the_hook_action_and_the_settings_file() {
        let run =
            Workspace::of("08-locked-hook", "repo-unlocked").explain(&["hook", "block-secrets"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.starts_with(
                "hook block-secrets\nsources, general → specific:\n  1. personal@1.0.0:hooks/hooks.yaml — hook block-secrets\n\nmerged result (.claude/settings.json):\n"
            ),
            "{}",
            run.stdout
        );
        assert!(run.stdout.contains("block-secrets.sh"), "{}", run.stdout);
        assert_eq!(run.stderr, "");
    }

    /// Spec §10.3 `alz explain`: `server <id>` is the action of an enabled server's definition, and
    /// the merged result is `.mcp.json`.
    #[test]
    fn explain_mcp_names_the_server_action_and_the_mcp_file() {
        let run = Workspace::of("09-mcp-allowlist", "repo").explain(&["mcp", "context7"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.starts_with(
                "mcp context7\nsources, general → specific:\n  1. personal@1.0.0:mcp.yaml — server context7\n\nmerged result (.mcp.json):\n"
            ),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("https://mcp.context7.com/mcp"),
            "{}",
            run.stdout
        );
        assert_eq!(run.stderr, "");
    }

    /// Spec §10.3 `alz explain`: the merged text loses every format character, a bidirectional
    /// control included, and keeps line feeds and tabs. A layer file cannot hold a raw control
    /// character (§4.3), so a carriage return and an escape cannot reach the merged file through
    /// the binary; `terminal_text.rs::file_text_keeps_line_feeds_and_tabs_and_loses_what_moves_the_cursor_or_hides_text`
    /// covers those on the function that strips it.
    #[test]
    fn explain_strips_format_and_bidirectional_characters_from_the_merged_text_but_keeps_tabs() {
        let scratch = ScratchDir::new().expect("a scratch directory");
        scratch.make_dirs("home").expect("made");
        scratch.make_dirs("repo/.layer/rules").expect("made");
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
            .write(
                "repo/.layer/rules/r.md",
                "---\npaths: [\"src/**\"]\n---\nbidi\u{202e}x\u{2066}y\u{200f}z\u{200b}w\u{feff}v\tafter tab\nlast\n",
            )
            .expect("written");
        let workspace = Workspace {
            repository: scratch.join("repo"),
            scratch,
        };
        let run = workspace.explain(&["rule", "r"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        let (_, merged) = run
            .stdout
            .split_once("merged result (.claude/rules/r.md):\n")
            .expect("the merged result");
        assert!(
            merged.contains("bidixyzwv\tafter tab\nlast\n"),
            "{merged:?}"
        );
        for hidden in ['\u{202e}', '\u{2066}', '\u{200f}', '\u{200b}', '\u{feff}'] {
            assert!(!run.stdout.contains(hidden), "{hidden:?} in {merged:?}");
        }
    }
}
