//! The command line contract of the skeleton: clap owns `--version`, `--help` and usage errors,
//! alz sets their streams and exit codes and sanitises what clap echoes (decision-26).

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one; an integration test crate is compiled with `cfg(test)` already.
#[cfg(test)]
mod tests {
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    fn alz() -> Command {
        cargo_bin_cmd!("alz")
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn version_prints_the_package_version_on_stdout_and_exits_zero() {
        for flag in ["--version", "-V"] {
            let output = alz().arg(flag).assert().success().get_output().clone();
            assert_eq!(
                text(&output.stdout),
                format!("alz {}\n", env!("CARGO_PKG_VERSION")),
                "stdout of alz {flag}"
            );
            assert_eq!(text(&output.stderr), "", "stderr of alz {flag}");
        }
    }

    #[test]
    fn help_prints_usage_on_stdout_and_exits_zero() {
        for flag in ["--help", "-h"] {
            let output = alz().arg(flag).assert().success().get_output().clone();
            assert!(
                text(&output.stdout).contains("Usage: alz"),
                "stdout of alz {flag}"
            );
            assert_eq!(text(&output.stderr), "", "stderr of alz {flag}");
        }
    }

    #[test]
    fn no_arguments_print_help_on_stderr_and_exit_two() {
        let output = alz().assert().code(2).get_output().clone();
        assert_eq!(text(&output.stdout), "");
        assert!(text(&output.stderr).contains("Usage: alz"));
    }

    #[test]
    fn a_usage_error_goes_to_stderr_and_exits_two() {
        let output = alz()
            .arg("--no-such-flag")
            .assert()
            .code(2)
            .get_output()
            .clone();
        assert_eq!(text(&output.stdout), "");
        let stderr = text(&output.stderr);
        assert!(stderr.contains("--no-such-flag"), "stderr: {stderr}");
        assert!(stderr.contains("Usage: alz"), "stderr: {stderr}");
    }

    #[test]
    fn a_usage_error_never_echoes_control_or_format_characters() {
        let hostile = "--bad\u{1b}[31mred\r\u{202e}flipped\u{7}";
        let output = alz().arg(hostile).assert().code(2).get_output().clone();
        let stderr = text(&output.stderr);
        assert!(stderr.contains("--bad[31mredflipped"), "stderr: {stderr:?}");
        for forbidden in ['\u{1b}', '\r', '\u{202e}', '\u{7}'] {
            assert!(
                !stderr.contains(forbidden),
                "stderr kept {forbidden:?}: {stderr:?}"
            );
        }
    }

    #[test]
    fn a_line_feed_in_an_argument_cannot_forge_a_diagnostic_line() {
        let output = alz()
            .arg("--x\nerror [fake] layer.yaml: forged")
            .assert()
            .code(2)
            .get_output()
            .clone();
        let stderr = text(&output.stderr);
        assert!(
            stderr.contains("'--xerror [fake] layer.yaml: forged'"),
            "stderr: {stderr:?}"
        );
        assert!(
            stderr.lines().all(|line| !line.starts_with("error [fake]")),
            "stderr: {stderr:?}"
        );
    }

    #[test]
    fn a_line_separator_in_an_argument_is_removed() {
        let output = alz()
            .arg("--x\u{2028}y\u{2029}z")
            .assert()
            .code(2)
            .get_output()
            .clone();
        let stderr = text(&output.stderr);
        assert!(stderr.contains("'--xyz'"), "stderr: {stderr:?}");
    }

    #[test]
    fn a_format_character_added_after_unicode_four_is_removed_from_an_argument() {
        // U+0890 (Arabic pound mark above), U+110BD (Kaithi number sign), U+1D173 (musical
        // symbol begin beam): format characters of the Cf table that an older list missed.
        let output = alz()
            .arg("--a\u{890}b\u{110bd}c\u{1d173}d")
            .assert()
            .code(2)
            .get_output()
            .clone();
        let stderr = text(&output.stderr);
        assert!(stderr.contains("'--abcd'"), "stderr: {stderr:?}");
    }
}
