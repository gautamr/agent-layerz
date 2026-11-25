//! `alz diff` (spec §10.3): the change a build would make as unified diffs, and no write.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// The `02-instructions` fixture of the acceptance suite in a scratch directory.
    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
            copy_tree(&fixtures.join("02-instructions"), &scratch.join("work"))
                .expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            let repository = scratch.join("work/repo");
            Self {
                scratch,
                repository,
            }
        }

        fn alz(&self, command: &str) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .timeout(Duration::from_secs(60))
                .arg(command)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn append(&self, name: &str, text: &str) {
            let path = format!("work/repo/{name}");
            let mut bytes = self.scratch.read(&path).expect("the generated file");
            bytes.extend_from_slice(text.as_bytes());
            self.scratch.write(&path, bytes).expect("written");
        }
    }

    #[test]
    fn shows_every_file_a_first_build_creates_and_the_lock_and_writes_nothing() {
        let workspace = Workspace::new();
        let before = read_tree(&workspace.repository).expect("read");
        let run = workspace.alz("diff");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .contains("--- a/AGENTS.md\n+++ b/AGENTS.md\n@@ -0,0 +1,")
        );
        assert!(run.stdout.contains("--- a/CLAUDE.md\n"));
        assert!(run.stdout.contains("--- a/layer.lock\n+++ b/layer.lock\n"));
        assert!(!run.stdout.contains("no changes"));
        assert_eq!(read_tree(&workspace.repository).expect("read"), before);
    }

    /// The paths of the `--- a/<path>` header lines of `diff`, in the order printed.
    fn headers(stdout: &str) -> Vec<&str> {
        stdout
            .lines()
            .filter_map(|line| line.strip_prefix("--- a/"))
            .collect()
    }

    /// Spec §10.3 `alz diff`: the changes come in the order of §4.3, the paths the build makes in
    /// name order, then `layer.lock`.
    #[test]
    fn prints_the_files_of_a_first_build_in_name_order_and_the_lock_last() {
        let workspace = Workspace::new();
        let run = workspace.alz("diff");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            headers(&run.stdout),
            ["AGENTS.md", "CLAUDE.md", "layer.lock"]
        );
    }

    /// Spec §10.3 `alz diff`: the paths the build makes (created and changed together) in name
    /// order, then the deleted ones, then `layer.lock`.
    #[test]
    fn prints_the_made_paths_in_name_order_then_the_deleted_ones_then_the_lock() {
        let workspace = Workspace::new();
        let agent = |name: &str| format!("---\nname: {name}\ndescription: d\n---\nbody\n");
        workspace
            .scratch
            .make_dirs("work/repo/.layer/agents")
            .expect("made");
        workspace
            .scratch
            .write("work/repo/.layer/agents/zz.md", agent("zz"))
            .expect("written");
        assert_eq!(workspace.alz("build").code, 0);
        workspace
            .scratch
            .remove_file("work/repo/.layer/agents/zz.md")
            .expect("removed");
        workspace
            .scratch
            .write("work/repo/.layer/agents/mm.md", agent("mm"))
            .expect("written");
        workspace.append("CLAUDE.md", "- an edit by hand\n");
        let run = workspace.alz("diff");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            headers(&run.stdout),
            [
                ".claude/agents/mm.md",
                "CLAUDE.md",
                ".claude/agents/zz.md",
                "layer.lock"
            ]
        );
    }

    #[test]
    fn prints_no_changes_on_a_built_repository() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("build").code, 0);
        let before = read_tree(&workspace.repository).expect("read");
        let run = workspace.alz("diff");
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (0, "no changes\n", "")
        );
        assert_eq!(read_tree(&workspace.repository).expect("read"), before);
    }

    #[test]
    fn shows_the_edit_to_a_generated_file_as_a_diff_that_undoes_it() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("build").code, 0);
        workspace.append("AGENTS.md", "- an edit by hand\n");
        let run = workspace.alz("diff");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .starts_with("--- a/AGENTS.md\n+++ b/AGENTS.md\n@@ "),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("\n-- an edit by hand\n"),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("--- a/CLAUDE.md"));
        assert!(!run.stdout.contains("no changes"));
    }

    #[test]
    fn a_change_of_the_executable_bit_alone_prints_its_headers_and_no_hunk() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("build").code, 0);
        workspace
            .scratch
            .set_mode("work/repo/AGENTS.md", 0o755)
            .expect("mode set");
        let run = workspace.alz("diff");
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (0, "--- a/AGENTS.md\n+++ b/AGENTS.md\n", "")
        );
    }

    #[test]
    fn a_missing_gitignore_block_is_not_shown_and_leaves_no_changes() {
        let workspace = Workspace::new();
        workspace.append("layer.yaml", "options:\n  commit_generated: false\n");
        assert_eq!(workspace.alz("build").code, 0);
        assert!(workspace.scratch.exists("work/repo/.gitignore"));
        workspace
            .scratch
            .remove_file("work/repo/.gitignore")
            .expect("removed");
        let run = workspace.alz("diff");
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (0, "no changes\n", "")
        );
        assert!(!workspace.scratch.exists("work/repo/.gitignore"));
    }

    #[test]
    fn never_prints_a_control_character_of_a_file() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz("build").code, 0);
        workspace.append("AGENTS.md", "- \u{1b}[31mred\u{202e}\r\n");
        let run = workspace.alz("diff");
        assert!(run.stdout.contains("\n-- [31mred\n"), "{:?}", run.stdout);
        assert!(!run.stdout.contains(['\u{1b}', '\r', '\u{202e}']));
    }

    #[test]
    fn notes_a_collision_on_standard_error_and_still_exits_0() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .write("work/repo/AGENTS.md", "mine\n")
            .expect("written");
        let run = workspace.alz("diff");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr.contains(
                "note [collision] AGENTS.md: is not generated by agent-layerz; alz build will refuse to overwrite it\n"
            ),
            "{}",
            run.stderr
        );
        assert!(!run.stdout.contains("--- a/AGENTS.md"));
    }

    #[test]
    fn fails_with_the_diagnostic_and_prints_no_diff_without_a_manifest() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .remove_file("work/repo/layer.yaml")
            .expect("removed");
        let run = workspace.alz("diff");
        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "");
        assert!(
            run.stderr.starts_with("error [no-manifest]"),
            "{}",
            run.stderr
        );
    }
}
