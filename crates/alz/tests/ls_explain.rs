//! `alz ls` and `alz explain` (spec §10.3, TASK-55.6.10): the sections and the layers that decide
//! them, the sources of an item general to specific, the merged file, and the lines of the errors.
//! The fixtures are the acceptance suite's.

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

    /// A scratch copy of a fixture of the acceptance suite, with its own home and cache.
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

        /// A repository whose one layer is `.layer/` with `files`.
        fn with_files(files: &[(&str, &str)]) -> Self {
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
            for (path, text) in files {
                scratch
                    .write(format!("repo/.layer/{path}"), text)
                    .expect("written");
            }
            Self {
                repository: scratch.join("repo"),
                scratch,
            }
        }

        fn alz(&self, arguments: &[&str]) -> Run {
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

    #[test]
    fn ls_prints_the_six_sections_with_the_layer_that_decides_each_item() {
        let run = Workspace::of("22-ls", "repo").alz(&["ls"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            run.stdout,
            format!(
                "agents (1)\n  {:<28} personal\nskills (0)\ncommands (0)\nrules (0)\nhooks (0)\nmcp (0)\n",
                "architect"
            )
        );
    }

    #[test]
    fn ls_lists_the_items_of_every_section_in_name_order() {
        let run = Workspace::of("01-determinism", "repo").alz(&["ls"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        let line = |name: &str, layer: &str| format!("  {name:<28} {layer}\n");
        assert!(
            run.stdout.contains(&format!(
                "skills (3)\n{}{}{}commands (1)\n{}",
                line("domain-glossary", "acme"),
                line("review-checklist", "personal"),
                line("stack-conventions", "shop-api"),
                line("learn", "personal"),
            )),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn ls_stops_with_exit_1_and_prints_no_listing_when_the_build_has_an_error() {
        let run = Workspace::of("04-tool-narrowing", "repo-widen").alz(&["ls"]);
        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "");
        assert!(run.stderr.contains("error [tool-widen]"), "{}", run.stderr);
    }

    #[test]
    fn explain_lists_the_sources_general_to_specific_then_the_merged_file() {
        let run = Workspace::of("17-rules", "repo").alz(&["explain", "rule", "testing"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.starts_with(
                "rule testing\nsources, general → specific:\n  \
                 1. personal@1.0.0:rules/testing.md — rule\n  \
                 2. shop-api:rules/testing.md — extend: paths +[e2e/**]\n\
                 \nmerged result (.claude/rules/testing.md):\n"
            ),
            "{}",
            run.stdout
        );
        assert!(run.stdout.contains("PERSONAL-TESTING"));
        assert!(run.stdout.contains("REPO-TESTING"));
    }

    #[test]
    fn explain_marks_a_definition_that_a_more_specific_layer_replaced_as_overridden() {
        let run = Workspace::of("17-rules", "repo").alz(&["explain", "rule", "backend"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(run.stdout.contains(
            "  1. personal@1.0.0:rules/backend.md — rule (overridden)\n  2. shop-api:rules/backend.md — rule\n"
        ));
    }

    #[test]
    fn explain_agent_never_marks_an_overlay_as_overridden() {
        let run = Workspace::of("03-agent-overlay", "repo").alz(&["explain", "agent", "architect"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.contains(
                "  1. personal@1.0.0:agents/architect.md — base\n  2. acme:agents/architect.overlay.md — extend: skills +[domain-glossary]; body section\n  3. shop-api:agents/architect.overlay.md — "
            ),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("(overridden)"));
    }

    #[test]
    fn explain_names_a_slot_stub_as_a_skill_with_the_source_slot_stub() {
        let run =
            Workspace::of("06-slots", "repo-loose").alz(&["explain", "skill", "domain-glossary"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.starts_with(
                "skill domain-glossary\nsources, general → specific:\n  1. slot stub — stub\n\nmerged result (.claude/skills/domain-glossary/SKILL.md):\n"
            ),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn explain_says_first_that_a_server_is_defined_but_not_enabled() {
        let run = Workspace::of("09-mcp-allowlist", "repo").alz(&["explain", "mcp", "playwright"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.starts_with(
                "mcp playwright is defined but not enabled (add it to mcp.enable in layer.yaml)\nmcp playwright\n"
            ),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn explain_finds_an_instruction_by_its_file_name() {
        let run =
            Workspace::of("02-instructions", "repo").alz(&["explain", "instruction", "00-core.md"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .contains("  1. personal@1.0.0:instructions/00-core.md — block\n")
        );
        assert!(run.stdout.contains("\nmerged result (AGENTS.md):\n"));
    }

    #[test]
    fn explain_a_name_the_merged_result_does_not_have_is_an_error_with_exit_1() {
        let run = Workspace::of("17-rules", "repo").alz(&["explain", "agent", "nobody"]);
        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            "error [explain] no agent named 'nobody' in the merged result\n"
        );
    }

    #[test]
    fn explain_strips_control_and_format_characters_from_the_name_it_echoes() {
        let run = Workspace::of("17-rules", "repo").alz(&[
            "explain",
            "rule",
            "a\u{1b}[2J\u{202e}b\nerror [forged] x",
        ]);
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [explain] no rule named 'a[2Jberror [forged] x' in the merged result\n"
        );
    }

    #[test]
    fn explain_strips_what_a_layer_wrote_from_the_merged_file() {
        let run = Workspace::with_files(&[(
            "rules/r.md",
            "---\npaths: [\"src/**\"]\n---\nline\u{202e}one\u{200b}\nline two\n",
        )])
        .alz(&["explain", "rule", "r"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(!run.stdout.contains('\u{202e}'), "{:?}", run.stdout);
        assert!(!run.stdout.contains('\u{200b}'), "{:?}", run.stdout);
        assert!(
            run.stdout.contains("lineone\nline two\n"),
            "{:?}",
            run.stdout
        );
    }

    #[test]
    fn explain_with_a_type_it_does_not_know_is_a_usage_error() {
        let run = Workspace::of("17-rules", "repo").alz(&["explain", "widget", "x"]);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_ne!(run.stderr, "");
    }

    #[test]
    fn explain_stops_with_exit_1_and_prints_nothing_when_the_build_has_an_error() {
        let run =
            Workspace::of("04-tool-narrowing", "repo-widen").alz(&["explain", "agent", "builder"]);
        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "");
        assert!(run.stderr.contains("error [tool-widen]"), "{}", run.stderr);
    }
}
