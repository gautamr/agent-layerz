//! The `claude` runner (spec §10.3, TASK-55.6.11): found as an absolute path on a validated
//! `PATH`, run with an argument array and a cleared environment, ended at its deadline with its
//! whole process group, and read within a cap. Each test writes a fake `claude` into a scratch
//! folder; nothing reads the real `~/.claude*`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::time::{Duration, Instant};

    use alz_core::env::EnvVars;
    use alz_io::{EnvSnapshot, Purpose, ToolError, ToolRunner};
    use alz_test_support::ScratchDir;

    fn snapshot(pairs: &[(&str, String)]) -> EnvSnapshot {
        EnvSnapshot::from_vars(
            pairs
                .iter()
                .map(|(name, value)| (OsString::from(name), OsString::from(value)))
                .collect::<EnvVars>(),
        )
    }

    fn runner_for(scratch: &ScratchDir, script: &str, purpose: Purpose) -> ToolRunner {
        scratch.write_executable("claude", script).expect("written");
        let env = snapshot(&[
            ("PATH", "/usr/bin:/bin".to_owned()),
            (
                "ALZ_CLAUDE_BIN",
                scratch.join("claude").to_string_lossy().into_owned(),
            ),
        ]);
        ToolRunner::claude(&env, purpose).expect("the fake is found")
    }

    fn arguments(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_program_that_exits_zero_gives_its_output() {
        let scratch = ScratchDir::new().expect("scratch");
        let runner = runner_for(
            &scratch,
            "#!/bin/sh\necho \"2.1.311 (Claude Code) $1\"\necho oops >&2\n",
            Purpose::Version,
        );
        let output = runner
            .run(&arguments(&["--version"]), scratch.path())
            .expect("runs");
        assert!(output.success);
        assert_eq!(output.stdout, b"2.1.311 (Claude Code) --version\n");
        assert_eq!(output.stderr, b"oops\n");
    }

    #[test]
    fn a_missing_program_is_not_found_and_so_is_a_file_without_an_execute_bit() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch.write("claude", "#!/bin/sh\n").expect("written");
        for configured in [
            scratch.join("absent"),
            scratch.join("claude"),
            "relative/claude".into(),
        ] {
            let env = snapshot(&[
                ("PATH", scratch.path().to_string_lossy().into_owned()),
                ("ALZ_CLAUDE_BIN", configured.to_string_lossy().into_owned()),
            ]);
            assert!(
                matches!(
                    ToolRunner::claude(&env, Purpose::Version),
                    Err(ToolError::NotFound)
                ),
                "{configured:?}"
            );
        }
    }

    #[test]
    fn the_name_claude_is_looked_up_on_absolute_path_entries_only() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dir("bin").expect("made");
        scratch
            .write_executable("bin/claude", "#!/bin/sh\n")
            .expect("written");
        let found = snapshot(&[(
            "PATH",
            format!(".:relative:{}", scratch.join("bin").display()),
        )]);
        assert_eq!(
            ToolRunner::claude(&found, Purpose::Version)
                .expect("found")
                .program(),
            scratch.join("bin/claude")
        );
        let only_relative = snapshot(&[("PATH", "bin:.".to_owned())]);
        assert!(matches!(
            ToolRunner::claude(&only_relative, Purpose::Version),
            Err(ToolError::NotFound)
        ));
    }

    #[test]
    fn a_program_that_outruns_its_deadline_is_ended_with_everything_it_started() {
        let scratch = ScratchDir::new().expect("scratch");
        let marker = scratch.join("child-survived");
        let script = format!(
            "#!/bin/sh\n(sleep 3; echo alive > '{}') &\nsleep 30\n",
            marker.display()
        );
        let runner = runner_for(&scratch, &script, Purpose::Version)
            .with_deadline(Duration::from_millis(300));
        let started = Instant::now();
        let outcome = runner.run(&arguments(&[]), scratch.path());
        assert!(
            matches!(outcome, Err(ToolError::TimedOut { .. })),
            "{outcome:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_secs(4));
        assert!(!marker.exists(), "the grandchild was ended with the group");
    }

    #[test]
    fn a_program_that_ends_by_itself_leaves_nothing_of_its_group_running() {
        let scratch = ScratchDir::new().expect("scratch");
        let marker = scratch.join("child-survived");
        let script = format!(
            "#!/bin/sh\n(sleep 6; echo alive > '{}') &\necho done\n",
            marker.display()
        );
        let runner = runner_for(&scratch, &script, Purpose::Version);
        let started = Instant::now();
        let output = runner.run(&arguments(&[]), scratch.path()).expect("runs");
        assert!(output.success);
        assert_eq!(output.stdout, b"done\n");
        // Well before the child's six seconds, with room for a loaded machine.
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "the readers finished when the group was ended, not when the child's sleep did"
        );
        std::thread::sleep(Duration::from_secs(7));
        assert!(
            !marker.exists(),
            "the child the program left behind was ended with its group"
        );
    }

    #[test]
    fn output_beyond_the_cap_is_dropped_and_the_program_is_not_stopped() {
        let scratch = ScratchDir::new().expect("scratch");
        let runner = runner_for(
            &scratch,
            "#!/bin/sh\nhead -c 300000 /dev/zero | tr '\\0' 'x'\nhead -c 300000 /dev/zero | tr '\\0' 'e' >&2\n",
            Purpose::Version,
        );
        let output = runner.run(&arguments(&[]), scratch.path()).expect("runs");
        assert!(output.success);
        assert_eq!(output.stdout.len(), 64 * 1024);
        assert_eq!(output.stderr.len(), 64 * 1024);
    }

    #[test]
    fn the_program_sees_only_the_variables_its_purpose_takes() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch
            .write_executable("claude", "#!/bin/sh\nenv | sort\n")
            .expect("written");
        let env = snapshot(&[
            ("PATH", "/usr/bin:/bin:rel".to_owned()),
            ("HOME", "/the/home".to_owned()),
            ("GITHUB_TOKEN", "secret".to_owned()),
            ("ANTHROPIC_API_KEY", "key".to_owned()),
            (
                "ALZ_CLAUDE_BIN",
                scratch.join("claude").to_string_lossy().into_owned(),
            ),
        ]);
        let seen = |purpose| {
            let output = ToolRunner::claude(&env, purpose)
                .expect("found")
                .run(&arguments(&[]), scratch.path())
                .expect("runs");
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        let version = seen(Purpose::Version);
        assert!(version.contains("HOME=/the/home"), "{version}");
        assert!(version.contains("PATH=/usr/bin:/bin\n"), "{version}");
        assert!(!version.contains("GITHUB_TOKEN"));
        assert!(!version.contains("ANTHROPIC_API_KEY"));
        assert!(!version.contains("ALZ_CLAUDE_BIN"));
        let probe = seen(Purpose::Probe);
        assert!(probe.contains("ANTHROPIC_API_KEY=key"), "{probe}");
        assert!(!probe.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn the_program_runs_in_the_directory_it_is_given_with_no_standard_input() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dir("here").expect("made");
        let runner = runner_for(&scratch, "#!/bin/sh\npwd\ncat\n", Purpose::Version);
        let output = runner
            .run(&arguments(&[]), &scratch.join("here"))
            .expect("runs");
        let printed = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(printed.trim_end().ends_with("/here"), "{printed}");
    }
}
