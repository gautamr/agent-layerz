//! The leak guard's rules about the `commit_identity` (§7.12) and `account` (§7.13) fields of a
//! public layer (spec §9.2, last two items), against `fixtures/10-leak-guard/layer-clean`: a
//! public layer's identity and account are public like any other line, so an address that is not
//! a no-reply or example one raises `leak-email`, and `forbid_patterns` apply to both fields.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// The clean public layer of fixture 10, with `extra` lines added to its `layer.yaml`.
    fn lint_with(extra: &str) -> Run {
        let scratch = ScratchDir::new().expect("a scratch directory");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/10-leak-guard/layer-clean");
        copy_tree(&fixture, &scratch.join("layer")).expect("the fixture is copied");
        scratch.make_dir("home").expect("made");
        let mut manifest = scratch.read_text("layer/layer.yaml").expect("the manifest");
        manifest.push_str(extra);
        scratch
            .write("layer/layer.yaml", manifest)
            .expect("written");
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(scratch.join("layer"))
            .env_clear()
            .env("PATH", scratch.join("none"))
            .env("HOME", scratch.join("home"))
            .timeout(Duration::from_secs(60))
            .arg("lint")
            .output()
            .expect("alz runs and ends in time");
        Run {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Spec §9.2: a `commit_identity` in a public layer is public; a no-reply address passes and
    /// any other address raises the `leak-email` warning, which does not fail the command and
    /// never prints the address.
    #[test]
    fn a_public_layers_commit_identity_warns_unless_its_address_is_a_no_reply_one() {
        let quiet = lint_with(
            "commit_identity:\n  name: Some One\n  email: 1234+someone@users.noreply.github.com\n",
        );
        assert_eq!(quiet.code, 0, "{}{}", quiet.stdout, quiet.stderr);
        assert_eq!(quiet.stderr, "");

        let warned =
            lint_with("commit_identity:\n  name: Some One\n  email: someone@mailbox.test\n");
        assert_eq!(warned.code, 0, "{}{}", warned.stdout, warned.stderr);
        assert!(
            warned.stderr.contains("warning [leak-email]"),
            "{}",
            warned.stderr
        );
        assert!(!warned.stderr.contains("someone@mailbox.test"));
        assert!(!warned.stdout.contains("someone@mailbox.test"));
    }

    /// Spec §9.2: `forbid_patterns` apply to the `account` and `commit_identity` lines of a public
    /// layer like to any other line; the error names the file and the entry, never the text.
    #[test]
    fn forbid_patterns_apply_to_the_account_and_identity_fields_of_a_public_layer() {
        for (extra, what) in [
            ("account: acme\n", "the account"),
            (
                "commit_identity:\n  name: Acme Person\n  email: p@users.noreply.github.com\n",
                "the identity",
            ),
        ] {
            let run = lint_with(extra);
            assert_eq!(run.code, 1, "{what}: {}{}", run.stdout, run.stderr);
            assert!(
                run.stderr.contains("error [leak-forbid]") && run.stderr.contains("#1"),
                "{what}: {}",
                run.stderr
            );
            assert!(
                !run.stderr.to_lowercase().contains("acme person")
                    && !run.stderr.contains("account: acme"),
                "{what} is quoted: {}",
                run.stderr
            );
        }
    }
}
