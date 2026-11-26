//! Spec §13 items 27 and 29 through the binary: `ref: 1.10` and `version: 1.10` are refused by
//! `alz lint` with the field named and the value never printed, `ref: "1.10"` is not, and a
//! settings or MCP file whose top-level key is not a target or a server id is an error naming the
//! file through `alz lint --layer` and writes no output through `alz build`. Every run has its
//! own home, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    fn repository(manifest: &str, layer_files: &[(&str, &str)]) -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["home", "config", "state", "cache", "repo/.layer"] {
            scratch.make_dirs(folder).expect("made");
        }
        scratch.write("repo/layer.yaml", manifest).expect("written");
        scratch
            .write(
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            )
            .expect("written");
        for (path, text) in layer_files {
            scratch
                .write(format!("repo/.layer/{path}"), text)
                .expect("written");
        }
        scratch
    }

    fn alz(scratch: &ScratchDir, arguments: &[&str]) -> Run {
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(scratch.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
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

    /// Spec §13 item 27: `alz lint` in a repository refuses `ref: 1.10` with `manifest`, naming
    /// `stack[<n>].ref` and printing neither value, and gets past `ref: "1.10"`.
    #[test]
    fn a_ref_that_is_a_number_is_refused_naming_the_field_and_a_quoted_one_is_not() {
        let unquoted = repository(
            "stack:\n  - source: path:./.layer\n    ref: 1.10\ntargets: [claude]\n",
            &[],
        );
        let run = alz(&unquoted, &["lint"]);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        let line = run
            .stderr
            .lines()
            .find(|line| line.starts_with("error [manifest]"))
            .unwrap_or_else(|| panic!("no manifest error: {}", run.stderr));
        assert!(line.contains("stack[0].ref"), "{line}");
        assert!(
            !run.stdout.contains("1.10") && !run.stderr.contains("1.1"),
            "{}{}",
            run.stdout,
            run.stderr
        );

        let quoted = repository(
            "stack:\n  - source: path:./.layer\n    ref: \"1.10\"\ntargets: [claude]\n",
            &[],
        );
        let run = alz(&quoted, &["lint"]);
        assert!(
            !run.stderr.contains("error [manifest]"),
            "{}{}",
            run.stdout,
            run.stderr
        );
    }

    /// Spec §13 item 27: `alz lint --layer` refuses `version: 1.10` with `layer`, naming
    /// `version`, and prints neither value.
    #[test]
    fn a_version_that_is_a_number_is_refused_with_the_error_layer_naming_version() {
        let scratch = repository(
            "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            &[],
        );
        scratch
            .write(
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\nversion: 1.10\n",
            )
            .expect("written");
        let run = alz(&scratch, &["lint", "--layer", ".layer"]);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        let line = run
            .stderr
            .lines()
            .find(|line| line.starts_with("error [layer]"))
            .unwrap_or_else(|| panic!("no layer error: {}", run.stderr));
        assert!(line.contains("version"), "{line}");
        assert!(!run.stderr.contains("1.1"), "{}", run.stderr);
    }

    /// Spec §13 item 29: a settings file whose top-level key is not a target, and an MCP file
    /// whose top-level key is not a server id, are errors naming the file through `alz lint
    /// --layer`, and `alz build` writes no output.
    #[test]
    fn a_top_level_key_that_is_not_a_target_or_a_server_id_is_an_error_and_a_build_writes_nothing()
    {
        let cases = [
            ("settings.yaml", "Gemini:\n  env:\n    A: x\n"),
            ("mcp.yaml", "Not An Id:\n  command: tool\n"),
        ];
        for (file, text) in cases {
            let scratch = repository(
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
                &[(file, text)],
            );
            let linted = alz(&scratch, &["lint", "--layer", ".layer"]);
            assert_eq!(linted.code, 1, "{file}: {}{}", linted.stdout, linted.stderr);
            assert!(
                linted
                    .stderr
                    .lines()
                    .any(|line| line.starts_with("error [") && line.contains(file)),
                "{file}: {}",
                linted.stderr
            );
            let before = read_tree(&scratch.join("repo")).expect("read");
            let built = alz(&scratch, &["build"]);
            assert_eq!(built.code, 1, "{file}: {}{}", built.stdout, built.stderr);
            assert!(
                built.stderr.lines().any(|line| line.contains(file)),
                "{file}: {}",
                built.stderr
            );
            assert_eq!(
                read_tree(&scratch.join("repo")).expect("read"),
                before,
                "{file}: the build wrote something"
            );
        }
    }
}
