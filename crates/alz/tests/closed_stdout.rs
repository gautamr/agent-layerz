//! A write to standard output that fails exits 1, not 70 (decision-26).
//!
//! This test has a test binary of its own: on macOS a pipe's ends are made close-on-exec only
//! after the pipe exists, so a child that a parallel test spawns in between inherits the read end,
//! keeps it open, and the write this test expects to fail succeeds. Alone in its process, no other
//! test can spawn a child at that moment.

// The test sits in a `mod tests` so that `clippy::tests_outside_test_module` holds here too.
#[cfg(test)]
mod tests {
    use assert_cmd::cargo::cargo_bin;

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// Runs `alz --version` with standard output redirected to `stdout`. `assert_cmd` cannot
    /// redirect a stream, so this one test starts the binary itself.
    #[expect(
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "assert_cmd has no way to redirect standard output"
    )]
    fn version_into(stdout: std::io::PipeWriter) -> std::process::Output {
        std::process::Command::new(cargo_bin!("alz"))
            .arg("--version")
            .stdout(stdout)
            .output()
            .expect("alz starts")
    }

    #[test]
    fn output_that_cannot_be_written_exits_one_and_not_seventy() {
        // The read end of the pipe is closed before alz starts, so its write fails with a
        // broken pipe; decision-26 keeps 70 for a bug in alz.
        let (reader, writer) = std::io::pipe().expect("the operating system creates a pipe");
        drop(reader);
        let output = version_into(writer);
        assert_eq!(output.status.code(), Some(1));
        assert!(
            text(&output.stderr).contains("alz: cannot write its output"),
            "stderr: {}",
            text(&output.stderr)
        );
    }
}
