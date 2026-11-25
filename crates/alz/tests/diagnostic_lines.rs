//! The diagnostic line of spec §10.1: `<level> [<code>] <where>: <message>`. The column of a YAML
//! error counts characters, a diagnostic that merging layers finds names `<layer>:<file>` with no
//! line and column, `permission-shadowed` has no `<where>`, `alz lint` leaves out `unresolved`,
//! and a value refused as a protected variable or as a secret is never printed. Each run has its
//! own `HOME` and `XDG_*` folders.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    const TWO_LAYERS: &str =
        "stack:\n  - source: path:./.layer-a\n  - source: path:./.layer\ntargets: [claude]\n";

    /// A repository with the repo layer `.layer` and `files` (from the repository root).
    fn repository(files: &[(&str, &str)]) -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["home", "config", "state", "cache"] {
            scratch.make_dirs(folder).expect("made");
        }
        let base = [
            (
                "layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            (
                ".layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
        ];
        for (path, text) in base.iter().chain(files) {
            let path = Path::new("repo").join(path);
            if let Some(parent) = path.parent() {
                scratch.make_dirs(parent).expect("made");
            }
            scratch.write(&path, text).expect("written");
        }
        scratch
    }

    fn alz(scratch: &ScratchDir, arguments: &[&str]) -> Run {
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(scratch.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
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

    /// The `<where>` of the one diagnostic line of `stderr` that starts with `prefix`: the text
    /// between the prefix and the first `: `.
    fn where_of(stderr: &str, prefix: &str) -> String {
        let line = stderr
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("no line starts with {prefix}: {stderr}"));
        let rest = line.strip_prefix(prefix).expect("the prefix");
        rest.split_once(": ")
            .map_or_else(|| rest.to_owned(), |(place, _)| place.to_owned())
    }

    /// The `<line>:<column>` that follows `layer.yaml:` in the error of linting `text` as a layer.
    fn position_of(text: &str) -> String {
        let scratch = ScratchDir::new().expect("a scratch directory");
        scratch.make_dirs("layer").expect("made");
        scratch.write("layer/layer.yaml", text).expect("written");
        scratch.make_dirs("home").expect("made");
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(scratch.path())
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .timeout(Duration::from_secs(60))
            .args(["lint", "--layer", "layer"])
            .output()
            .expect("alz runs and ends in time");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        let line = stderr.lines().next().expect("one diagnostic");
        let after = line
            .split_once("layer:layer.yaml:")
            .unwrap_or_else(|| panic!("no position in {line}"))
            .1;
        after.splitn(3, ':').take(2).collect::<Vec<_>>().join(":")
    }

    /// Spec §10.1 "Diagnostic line": a YAML error adds `:<line>:<column>`, with the column counted
    /// in characters. Text that has more bytes than characters before the error gives the same
    /// column as the same text in ASCII, whether the multi-byte characters are on the line of the
    /// error or on a line before it.
    #[test]
    fn a_column_counts_characters_and_not_bytes() {
        let ascii = position_of("# eee\nname: eee: x\n");
        assert_ne!(ascii, "", "a position is printed");
        assert_eq!(position_of("# \u{e9}\u{e9}\u{e9}\nname: eee: x\n"), ascii);
        assert_eq!(
            position_of("# eee\nname: \u{e9}\u{e9}\u{e9}: x\n"),
            position_of("# eee\nname: eee: x\n")
        );
        assert_eq!(
            position_of("# \u{1f600}\nname: \u{1f600}\u{1f600}: x\n"),
            position_of("# e\nname: ee: x\n")
        );
    }

    /// Spec §10.1 "Merge diagnostics": a diagnostic found by merging layers names the file as
    /// `<layer>:<file>`, with no line and no column.
    #[test]
    fn a_merge_diagnostic_names_the_layer_and_the_file_without_a_line_or_a_column() {
        let overlay = repository(&[(
            ".layer/agents/zz.overlay.md",
            "---\nextends: zz\n---\nmore\n",
        )]);
        let run = alz(&overlay, &["build"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            where_of(&run.stderr, "error [unresolved] "),
            "shop-api:agents/zz.overlay.md"
        );

        let protected = repository(&[(
            ".layer/settings.yaml",
            "claude:\n  env:\n    CLAUDE_CONFIG_DIR: /x\n",
        )]);
        let run = alz(&protected, &["build"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            where_of(&run.stderr, "error [protected-variable] "),
            "shop-api:settings.yaml"
        );

        let secret = repository(&[
            ("layer.yaml", TWO_LAYERS),
            (
                ".layer-a/layer.yaml",
                "name: base\nkind: personal\nvisibility: public\n",
            ),
            (
                ".layer-a/instructions/00-core.md",
                "token ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
            ),
        ]);
        let run = alz(&secret, &["build"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            where_of(&run.stderr, "error [leak-secret] "),
            "base:instructions/00-core.md"
        );
    }

    /// Spec §10.1 "Merge diagnostics": `permission-shadowed` is the one merge warning without
    /// `<where>`, because the allow rule and the deny rule can come from different layers.
    #[test]
    fn permission_shadowed_is_a_warning_without_a_where() {
        let scratch = repository(&[(
            ".layer/permissions.yaml",
            "allow: [\"bash:ls\"]\ndeny: [\"bash:ls\"]\n",
        )]);
        let run = alz(&scratch, &["build"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr.lines().any(|line| line
                == "warning [permission-shadowed] allow rule 'bash:ls' is also denied; deny wins"),
            "{}",
            run.stderr
        );
    }

    /// Spec §10.1 "Merge diagnostics": `alz lint` checks each layer alone and leaves out
    /// `unresolved`, which `alz build` reports.
    #[test]
    fn lint_leaves_out_unresolved_and_build_reports_it() {
        let scratch = repository(&[(
            ".layer/agents/zz.overlay.md",
            "---\nextends: zz\n---\nmore\n",
        )]);
        let linted = alz(&scratch, &["lint", "--layer", ".layer"]);
        assert_eq!(
            (linted.code, linted.stderr.as_str(), linted.stdout.as_str()),
            (0, "", "lint ok: shop-api\n")
        );
        let built = alz(&scratch, &["build"]);
        assert_eq!(built.code, 1);
        assert!(
            built.stderr.contains("error [unresolved] "),
            "{}",
            built.stderr
        );
    }

    /// Spec §10.1 "`alz lint` in a repository": when the lock is invalid no layer is resolved and
    /// no summary line is printed, whatever the manifest says.
    #[test]
    fn an_invalid_lock_stops_the_stack_lint_with_no_summary_line() {
        let scratch = repository(&[("layer.lock", "{\"lockVersion\":2}")]);
        let run = alz(&scratch, &["lint"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(run.stdout, "", "no summary line");
        assert!(
            run.stderr.starts_with("error [lock] layer.lock"),
            "{}",
            run.stderr
        );
    }

    /// Spec §10.1 "Merge diagnostics": a value refused as a protected variable or as a secret is
    /// never printed, in a diagnostic or on standard output.
    #[test]
    fn a_refused_protected_variable_or_secret_value_is_never_printed() {
        let protected = repository(&[(
            ".layer/settings.yaml",
            "claude:\n  env:\n    CLAUDE_CONFIG_DIR: /value-that-must-stay-hidden\n",
        )]);
        let run = alz(&protected, &["build"]);
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains("[protected-variable]"),
            "{}",
            run.stderr
        );
        let secret = repository(&[
            ("layer.yaml", TWO_LAYERS),
            (
                ".layer-a/layer.yaml",
                "name: base\nkind: personal\nvisibility: public\n",
            ),
            (
                ".layer-a/instructions/00-core.md",
                "token ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
            ),
        ]);
        let leaked = alz(&secret, &["build"]);
        assert_eq!(leaked.code, 1);
        assert!(leaked.stderr.contains("[leak-secret]"), "{}", leaked.stderr);
        for (hidden, run) in [
            ("/value-that-must-stay-hidden", &run),
            ("ghp_abcdefghijklmnopqrstuvwxyz0123456789", &leaked),
        ] {
            assert!(!run.stderr.contains(hidden), "{}", run.stderr);
            assert!(!run.stdout.contains(hidden), "{}", run.stdout);
        }
    }
}
