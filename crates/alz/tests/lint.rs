//! `alz lint`: a layer through `--layer`, and a repository's stack manifest and lock in the
//! working directory (spec §10, §10.1). Files are planted in a scratch directory, and every run
//! has a timeout, so a command that blocks on a pipe fails the test instead of hanging it.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one; an integration test crate is compiled with `cfg(test)` already.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const LAYER: &str = "name: personal\nkind: personal\nvisibility: private\n";
    const MANIFEST: &str = "stack:\n  - source: path:../personal\n";
    const LOCK: &str = r#"{"lockVersion":1,"visibility":"private","layers":[],"files":{}}"#;

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    fn scratch() -> ScratchDir {
        ScratchDir::new().expect("a scratch directory")
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

    fn lint_layer(directory: &ScratchDir, layer: &str) -> Run {
        alz(directory.path(), &["lint", "--layer", layer])
    }

    /// A scratch directory with a layer folder `layer` whose `layer.yaml` is `yaml`.
    fn with_layer(yaml: impl AsRef<[u8]>) -> ScratchDir {
        let directory = scratch();
        directory.make_dir("layer").expect("made");
        directory.write("layer/layer.yaml", yaml).expect("written");
        directory
    }

    #[test]
    fn a_valid_layer_lints_with_an_empty_stderr_and_a_summary_on_stdout() {
        let directory = with_layer(LAYER);
        let run = lint_layer(&directory, "layer");
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal\n");
    }

    #[test]
    fn every_schema_error_of_a_layer_is_reported_and_none_prints_a_value() {
        let directory = with_layer("name: Secret-Value\nkind: 7\nbogus: hunter2\n");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        let errors: Vec<&str> = run.stderr.lines().collect();
        assert!(errors.len() >= 3, "stderr: {}", run.stderr);
        assert!(
            errors
                .iter()
                .all(|line| line.starts_with("error [layer] layer:layer.yaml:")),
            "stderr: {}",
            run.stderr
        );
        assert!(run.stderr.contains("'bogus'"), "stderr: {}", run.stderr);
        for text in [&run.stdout, &run.stderr] {
            assert!(!text.contains("hunter2"), "output: {text}");
            assert!(!text.contains("Secret-Value"), "output: {text}");
        }
        assert_eq!(
            run.stdout,
            format!("lint failed: {} errors in layer\n", errors.len())
        );
    }

    #[test]
    fn one_error_is_summarised_in_the_singular() {
        let directory = with_layer("name: a\nkind: personal\n");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "lint failed: 1 error in layer\n");
    }

    #[test]
    fn a_declaration_that_breaks_its_rule_names_the_field_and_not_the_value() {
        let directory = with_layer(format!("{LAYER}account: ~/secret-dir\n"));
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains("error [layer] layer:layer.yaml:4:"),
            "stderr: {}",
            run.stderr
        );
        assert!(run.stderr.contains("account"), "stderr: {}", run.stderr);
        assert!(!run.stderr.contains("secret-dir"), "stderr: {}", run.stderr);
    }

    #[test]
    fn a_missing_layer_yaml_is_the_error_layer() {
        let directory = scratch();
        directory.make_dir("layer").expect("made");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [layer] missing layer.yaml in layer 'layer'\n"
        );
        assert_eq!(run.stdout, "lint failed: 1 error in layer\n");
    }

    #[test]
    fn a_layer_directory_that_does_not_exist_is_the_error_layer() {
        let directory = scratch();
        let run = lint_layer(&directory, "nowhere");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [layer] missing layer.yaml in layer 'nowhere'\n"
        );
    }

    const NOT_REGULAR: &str = "error [layer] layer.yaml in layer 'layer' must be a regular file, not a link or a folder\n";

    #[test]
    fn a_layer_yaml_that_is_a_link_is_refused_and_never_read() {
        let directory = scratch();
        directory.make_dir("layer").expect("made");
        directory.write("real.yaml", LAYER).expect("written");
        directory
            .make_link("layer/layer.yaml", "../real.yaml")
            .expect("linked");
        let run = lint_layer(&directory, "layer");
        assert_eq!((run.code, run.stderr.as_str()), (1, NOT_REGULAR));
        assert!(!run.stdout.contains("personal"), "stdout: {}", run.stdout);
    }

    #[test]
    fn a_layer_yaml_that_is_a_folder_is_refused() {
        let directory = scratch();
        directory.make_dir("layer").expect("made");
        directory.make_dir("layer/layer.yaml").expect("made");
        let run = lint_layer(&directory, "layer");
        assert_eq!((run.code, run.stderr.as_str()), (1, NOT_REGULAR));
    }

    #[test]
    fn a_layer_yaml_that_is_a_pipe_is_refused_without_blocking() {
        let directory = scratch();
        directory.make_dir("layer").expect("made");
        directory.make_fifo("layer/layer.yaml").expect("a pipe");
        let run = lint_layer(&directory, "layer");
        assert_eq!((run.code, run.stderr.as_str()), (1, NOT_REGULAR));
    }

    #[test]
    fn a_layer_yaml_past_one_mebibyte_is_the_error_yaml() {
        let at_cap = format!("{LAYER}#{}\n", "a".repeat(1_048_576 - LAYER.len() - 2));
        assert_eq!(at_cap.len(), 1_048_576);
        let past_cap = format!("{at_cap} ");
        let ok = lint_layer(&with_layer(at_cap), "layer");
        assert_eq!((ok.code, ok.stderr.as_str()), (0, ""));
        let run = lint_layer(&with_layer(past_cap), "layer");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [yaml] layer:layer.yaml: the file is larger than 1 MiB\n"
        );
    }

    #[test]
    fn a_layer_yaml_that_is_not_utf8_is_the_error_yaml_at_the_bad_byte() {
        let directory = with_layer(b"name: \xff\n".as_slice());
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [yaml] layer:layer.yaml:1:7: the file is not valid UTF-8\n"
        );
    }

    #[test]
    fn a_hostile_directory_name_never_reaches_the_output_unstripped() {
        let directory = scratch();
        let run = lint_layer(&directory, "x\u{1b}[31m\nerror [fake] y\u{202e}");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [layer] missing layer.yaml in layer 'x[31merror [fake] y'\n"
        );
        assert_eq!(run.stdout.lines().count(), 1);
        assert!(!run.stdout.contains(['\u{1b}', '\u{202e}']));
    }

    #[test]
    fn a_stack_manifest_is_not_a_layer_for_lint_layer() {
        let directory = with_layer(MANIFEST);
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert!(run.stderr.starts_with("error [layer]"), "{}", run.stderr);
    }

    #[test]
    fn lint_without_layer_lints_the_working_directory_as_a_layer() {
        let directory = scratch();
        directory.write("layer.yaml", LAYER).expect("written");
        let run = alz(directory.path(), &["lint"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal\n");
    }

    #[test]
    fn lint_in_a_directory_without_layer_yaml_is_the_error_layer() {
        let directory = scratch();
        let run = alz(directory.path(), &["lint"]);
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [layer] missing layer.yaml in layer '.'\n"
        );
    }

    #[test]
    fn a_valid_manifest_and_lock_resolve_the_stack_and_print_the_summary() {
        let directory = scratch();
        directory.make_dir("personal").expect("made");
        directory.make_dir("repo").expect("made");
        directory
            .write("personal/layer.yaml", LAYER)
            .expect("written");
        directory
            .write("repo/layer.yaml", MANIFEST)
            .expect("written");
        directory.write("repo/layer.lock", LOCK).expect("written");
        let run = alz(&directory.path().join("repo"), &["lint"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal\n");
    }

    #[test]
    fn a_valid_manifest_without_a_lock_resolves_the_stack_too() {
        let directory = scratch();
        directory.make_dir("personal").expect("made");
        directory.make_dir("repo").expect("made");
        directory
            .write("personal/layer.yaml", LAYER)
            .expect("written");
        directory
            .write("repo/layer.yaml", MANIFEST)
            .expect("written");
        let run = alz(&directory.path().join("repo"), &["lint"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal\n");
    }

    #[test]
    fn a_stack_whose_layer_is_missing_is_the_error_source_with_a_summary() {
        let directory = scratch();
        directory.make_dir("repo").expect("made");
        directory
            .write("repo/layer.yaml", MANIFEST)
            .expect("written");
        let run = alz(&directory.path().join("repo"), &["lint"]);
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [source] layer directory not found for 'path:../personal'\n"
        );
        assert_eq!(run.stdout, "lint failed: 1 error in path:../personal\n");
    }

    #[test]
    fn manifest_errors_are_reported_together() {
        let directory = scratch();
        directory
            .write(
                "layer.yaml",
                "stack:\n  - source: path:../a\n    ref: [x]\ntargets: [cursor]\nbogus: 1\n",
            )
            .expect("written");
        let run = alz(directory.path(), &["lint"]);
        assert_eq!(run.code, 1);
        let lines: Vec<&str> = run.stderr.lines().collect();
        assert!(lines.len() >= 3, "stderr: {}", run.stderr);
        assert!(
            lines
                .iter()
                .all(|line| line.starts_with("error [manifest] layer.yaml:")),
            "stderr: {}",
            run.stderr
        );
        assert!(
            run.stderr.contains("stack[0].ref"),
            "stderr: {}",
            run.stderr
        );
    }

    #[test]
    fn a_manifest_error_and_a_lock_error_are_both_reported() {
        let directory = scratch();
        directory
            .write("layer.yaml", "stack: 1\n")
            .expect("written");
        directory.write("layer.lock", "{").expect("written");
        let run = alz(directory.path(), &["lint"]);
        assert_eq!(run.code, 1);
        assert!(run.stderr.contains("error [manifest]"), "{}", run.stderr);
        assert!(
            run.stderr.contains("error [lock] layer.lock:1:"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn an_invalid_lock_with_a_valid_manifest_is_the_error_lock() {
        let directory = scratch();
        directory.write("layer.yaml", MANIFEST).expect("written");
        directory
            .write("layer.lock", r#"{"lockVersion":2}"#)
            .expect("written");
        let run = alz(directory.path(), &["lint"]);
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [lock] layer.lock"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn a_lock_that_is_a_link_or_a_pipe_is_the_error_lock_and_never_blocks() {
        for plant in [
            |directory: &ScratchDir| directory.make_link("layer.lock", "layer.yaml"),
            |directory: &ScratchDir| directory.make_fifo("layer.lock"),
        ] {
            let directory = scratch();
            directory.write("layer.yaml", MANIFEST).expect("written");
            plant(&directory).expect("planted");
            let run = alz(directory.path(), &["lint"]);
            assert_eq!(run.code, 1);
            assert_eq!(
                run.stderr,
                "error [lock] layer.lock: layer.lock must be a regular file, not a link or a folder\n"
            );
        }
    }

    #[test]
    fn a_lock_past_one_mebibyte_is_the_error_lock() {
        let directory = scratch();
        directory.write("layer.yaml", MANIFEST).expect("written");
        directory
            .write("layer.lock", vec![b' '; 1_048_577])
            .expect("written");
        let run = alz(directory.path(), &["lint"]);
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [lock] layer.lock: the file is larger than 1 MiB\n"
        );
    }

    // --- the leak guard (spec §9.2, TASK-55.5.14) ------------------------------------------------

    const PUBLIC_LAYER: &str = "name: shared\nkind: personal\nvisibility: public\n";

    /// A token assembled at run time, so that no file of the repository holds a token-shaped string.
    fn token() -> String {
        format!("{}_{}", "ghp", "aB3".repeat(13))
    }

    #[test]
    fn a_public_layer_with_a_forbidden_pattern_fails_naming_the_file_and_the_entry_only() {
        let directory = with_layer(format!(
            "{PUBLIC_LAYER}forbid_patterns:\n  - \"never-seen\"\n  - \"(?i)hidden-client\"\n"
        ));
        directory.make_dir("layer/skills").expect("made");
        directory
            .write("layer/skills/a.md", "Bill HIDDEN-CLIENT monthly\n")
            .expect("written");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [leak-forbid] shared:skills/a.md: matches forbid_patterns entry #2\n"
        );
        assert_eq!(run.stdout, "lint failed: 1 error in shared\n");
    }

    #[test]
    fn a_secret_in_a_public_layer_is_named_by_its_kind_and_never_printed() {
        let directory = with_layer(PUBLIC_LAYER);
        let secret = token();
        directory
            .write("layer/ci.md", format!("- CI: {secret}\n"))
            .expect("written");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert!(
            run.stderr
                .contains("error [leak-secret] shared:ci.md: looks like a secret (GitHub token)"),
            "{}",
            run.stderr
        );
        assert!(!run.stderr.contains(&secret) && !run.stdout.contains(&secret));
    }

    #[test]
    fn an_email_and_a_private_host_warn_and_the_exit_code_stays_zero() {
        let directory = with_layer(PUBLIC_LAYER);
        directory
            .write("layer/a.md", "ask dev@acme-widgets.io or build.corp\n")
            .expect("written");
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 0);
        assert_eq!(
            run.stderr,
            "warning [leak-email] shared:a.md: contains an email address; check that it may be public\n\
             warning [leak-host] shared:a.md: contains what looks like a private host name; check that it may be public\n"
        );
    }

    #[test]
    fn an_unsupported_entry_is_refused_by_its_number_and_a_private_layer_is_not_checked() {
        let patterns = "forbid_patterns:\n  - \"a(?=hidden-text)\"\n";
        let directory = with_layer(format!("{PUBLIC_LAYER}{patterns}"));
        let run = lint_layer(&directory, "layer");
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #1 uses look-around, which is not supported by the pattern syntax\n"
        );
        let directory = with_layer(format!("{LAYER}{patterns}"));
        directory.write("layer/a.md", token()).expect("written");
        let run = lint_layer(&directory, "layer");
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    }

    #[test]
    fn lint_arguments_are_checked_by_clap() {
        let directory = scratch();
        let run = alz(directory.path(), &["lint", "--no-such-flag"]);
        assert_eq!((run.code, run.stdout.as_str()), (2, ""));
        assert!(run.stderr.contains("--no-such-flag"), "{}", run.stderr);
        let run = alz(directory.path(), &["lint", "--layer"]);
        assert_eq!((run.code, run.stdout.as_str()), (2, ""));
        let run = alz(directory.path(), &["lint", "--help"]);
        assert_eq!(run.code, 0);
        assert!(run.stdout.contains("--layer"), "{}", run.stdout);
    }
}
