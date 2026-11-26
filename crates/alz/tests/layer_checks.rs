//! `alz lint` runs the reader of every kind of item on each layer alone (spec §10, TASK-55.5.15):
//! one layer through `--layer`, and each layer of a stack. A reference to a more general layer
//! (`unresolved`) is dropped, and the summary line closes the output.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const LAYER: &str = "name: l\nkind: client\nvisibility: private\n";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    fn alz(directory: &Path, arguments: &[&str]) -> Run {
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(directory)
            .env("HOME", directory)
            .timeout(Duration::from_secs(20))
            .args(arguments)
            .output()
            .expect("alz runs and ends in time");
        Run {
            code: output.status.code().expect("alz ends with an exit code"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// A scratch directory with the layer `layer/` holding `layer_yaml` and the `files`.
    fn with_layer(layer_yaml: &str, files: &[(&str, &str)]) -> ScratchDir {
        let directory = ScratchDir::new().expect("a scratch directory");
        directory.make_dir("layer").expect("made");
        directory
            .write("layer/layer.yaml", layer_yaml)
            .expect("written");
        for (path, text) in files {
            let full = format!("layer/{path}");
            if let Some((folder, _)) = full.rsplit_once('/') {
                directory.make_dirs(folder).expect("made");
            }
            directory.write(&full, text).expect("written");
        }
        directory
    }

    fn lint(files: &[(&str, &str)]) -> Run {
        lint_layer(LAYER, files)
    }

    fn lint_layer(layer_yaml: &str, files: &[(&str, &str)]) -> Run {
        let directory = with_layer(layer_yaml, files);
        alz(directory.path(), &["lint", "--layer", "layer"])
    }

    fn assert_refused(run: &Run, line_start: &str) {
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stderr.lines().any(|line| line.starts_with(line_start)),
            "no line starts with {line_start:?} in:\n{}",
            run.stderr
        );
        assert_eq!(run.stdout, "lint failed: 1 error in l\n");
    }

    #[test]
    fn an_instruction_with_an_unknown_mode_is_an_error_of_the_layer_alone() {
        let run = lint(&[("instructions/a.md", "---\nmode: bogus\n---\ntext\n")]);
        assert_refused(&run, "error [instructions] l:instructions/a.md:2:7: ");
    }

    #[test]
    fn a_rule_with_an_unknown_field_is_an_error() {
        let run = lint(&[("rules/r.md", "---\nbogus: 1\n---\ntext\n")]);
        assert_refused(&run, "error [rule] l:rules/r.md:2:1: ");
    }

    #[test]
    fn an_agent_with_an_unknown_field_is_an_error() {
        let run = lint(&[(
            "agents/a.md",
            "---\nname: a\ndescription: d\nbogus: 1\n---\nbody\n",
        )]);
        assert_refused(&run, "error [agent] l:agents/a.md:4:1: ");
    }

    #[test]
    fn an_overlay_whose_extends_does_not_match_its_file_is_an_error() {
        let run = lint(&[("agents/b.overlay.md", "---\nextends: other\n---\n")]);
        assert_refused(&run, "error [agent-overlay] l:agents/b.overlay.md: ");
    }

    #[test]
    fn a_command_without_a_description_is_an_error() {
        let run = lint(&[("commands/c.md", "no frontmatter\n")]);
        assert_refused(&run, "error [command] l:commands/c.md:1:1: ");
    }

    #[test]
    fn a_hook_file_with_a_problem_is_an_error() {
        let run = lint(&[("hooks/hooks.yaml", "- bogus: 1\n")]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stderr
                .lines()
                .any(|line| line.starts_with("error [hooks] l:hooks/hooks.yaml:")),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn permissions_that_are_not_lists_are_an_error() {
        let run = lint(&[("permissions.yaml", "allow: 3\n")]);
        assert_refused(&run, "error [permissions] l:permissions.yaml:1:8: ");
    }

    #[test]
    fn a_settings_file_with_a_key_that_is_not_a_target_is_an_error() {
        let run = lint(&[("settings.yaml", "NotTarget: 1\n")]);
        assert_refused(&run, "error [settings] l:settings.yaml:1:1: ");
    }

    #[test]
    fn an_mcp_file_with_a_definition_that_is_not_a_mapping_is_an_error() {
        let run = lint(&[("mcp.yaml", "servers: 1\n")]);
        assert_refused(&run, "error [mcp] l:mcp.yaml:1:10: ");
    }

    #[test]
    fn a_models_file_with_aliases_that_are_not_a_mapping_is_an_error() {
        let run = lint(&[("models.yaml", "aliases: 3\n")]);
        assert_refused(&run, "error [models] l:models.yaml:1:10: ");
    }

    #[test]
    fn a_passthrough_file_that_names_a_protected_variable_is_an_error() {
        let run = lint(&[(
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"CODEX_HOME\": \"x\"}}",
        )]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stderr
                .contains("error [protected-variable] l:targets/claude/.claude/settings.json"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn a_slot_the_layer_provides_without_a_skill_and_a_slot_name_that_is_not_kebab_case_are_errors()
    {
        let run = lint_layer(
            "name: l\nkind: client\nvisibility: private\nprovides_slots: [ghost]\nrequires_slots: [Bad_Name]\n",
            &[],
        );
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            run.stderr.lines().collect::<Vec<_>>(),
            [
                "error [slot] l:layer.yaml: requires_slots entry Bad_Name is not a kebab-case skill name",
                "error [slot] l:layer.yaml: provides_slots lists ghost, but the layer has no such skill",
            ]
        );
        assert_eq!(run.stdout, "lint failed: 2 errors in l\n");
    }

    #[test]
    fn a_provided_slot_that_a_skill_of_the_layer_fills_is_not_an_error() {
        let run = lint_layer(
            "name: l\nkind: client\nvisibility: private\nprovides_slots: [review]\n",
            &[(
                "skills/review/SKILL.md",
                "---\nname: review\ndescription: d\n---\nbody\n",
            )],
        );
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    }

    #[test]
    fn what_needs_a_more_general_layer_is_not_reported_for_a_layer_alone() {
        let run = lint(&[
            ("instructions/a.md", "---\nmode: replace\n---\ntext\n"),
            ("agents/b.overlay.md", "---\nextends: b\n---\n"),
            ("hooks/hooks.yaml", "- id: nothing\n  disabled: true\n"),
            ("models.yaml", "roles:\n  review: ghost\ndefault: ghost\n"),
        ]);
        assert!(
            !run.stderr.contains("unresolved"),
            "an unresolved diagnostic was reported:\n{}",
            run.stderr
        );
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(run.stdout, "lint ok: l\n");
    }

    #[test]
    fn every_problem_of_every_kind_is_reported_in_one_run_before_the_summary() {
        let run = lint(&[
            ("instructions/a.md", "---\nmode: bogus\n---\ntext\n"),
            ("rules/r.md", "---\nbogus: 1\n---\ntext\n"),
            ("permissions.yaml", "allow: 3\n"),
        ]);
        let kinds: Vec<&str> = run
            .stderr
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect();
        assert_eq!(kinds, ["[instructions]", "[rule]", "[permissions]"]);
        assert_eq!(run.stdout, "lint failed: 3 errors in l\n");
    }

    #[test]
    fn each_layer_of_a_stack_is_checked_alone_and_the_summary_names_them_all() {
        let directory = ScratchDir::new().expect("a scratch directory");
        directory.make_dirs("personal").expect("made");
        directory
            .write(
                "personal/layer.yaml",
                "name: personal\nkind: personal\nvisibility: private\n",
            )
            .expect("written");
        directory.make_dirs("personal/rules").expect("made");
        directory
            .write("personal/rules/r.md", "---\nbogus: 1\n---\ntext\n")
            .expect("written");
        directory
            .make_dirs("repo/.layer/instructions")
            .expect("made");
        directory
            .write(
                "repo/.layer/layer.yaml",
                "name: repo\nkind: repo\nvisibility: private\n",
            )
            .expect("written");
        directory
            .write(
                "repo/.layer/instructions/a.md",
                "---\nmode: replace\n---\ntext\n",
            )
            .expect("written");
        directory
            .write(
                "repo/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:./.layer\n",
            )
            .expect("written");
        let run = alz(&directory.path().join("repo"), &["lint"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            run.stderr
                .lines()
                .map(|line| line.split(':').next().unwrap_or_default())
                .collect::<Vec<_>>(),
            ["error [rule] personal"]
        );
        assert_eq!(run.stdout, "lint failed: 1 error in personal, repo\n");
    }
}
