//! The command line contract of the skeleton: clap owns `--version`, `--help` and usage errors,
//! alz sets their streams and exit codes and sanitises what clap echoes (decision-26).

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
        assert!(stderr.contains("--badredflipped"), "stderr: {stderr:?}");
        for forbidden in ['\u{1b}', '\r', '\u{202e}', '\u{7}'] {
            assert!(
                !stderr.contains(forbidden),
                "stderr kept {forbidden:?}: {stderr:?}"
            );
        }
    }
}
