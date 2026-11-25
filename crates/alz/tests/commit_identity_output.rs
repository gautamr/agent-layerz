//! What a declared commit identity does and does not write (spec §7.12 "Attribution is
//! instruction-only" and "Generated Commits rule"), against `fixtures/26-commit-identity`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    /// The fixture built with no git on the PATH (the identity step only warns), and the files it wrote.
    fn built() -> std::collections::BTreeMap<String, String> {
        let scratch = ScratchDir::new().expect("a scratch directory");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/26-commit-identity");
        copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
        scratch.make_dir("home").expect("made");
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(scratch.join("work/repo"))
            .env_clear()
            .env("PATH", scratch.join("none"))
            .env("HOME", scratch.join("home"))
            .timeout(Duration::from_secs(60))
            .arg("build")
            .output()
            .expect("alz runs and ends in time");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        read_tree(&scratch.join("work/repo"))
            .expect("read")
            .into_iter()
            .filter(|(path, _)| !path.starts_with(".layer/") && path != "layer.yaml")
            .map(|(path, file)| (path, String::from_utf8_lossy(&file.bytes).into_owned()))
            .collect()
    }

    /// Spec §7.12 "Attribution is instruction-only": alz sets no Claude Code or Codex attribution
    /// setting for the trailer, so no generated file but the instructions mentions one, and the
    /// instructions carry the rule.
    #[test]
    fn a_declared_identity_is_written_as_instructions_only_and_sets_no_attribution_setting() {
        let files = built();
        assert!(files.contains_key("AGENTS.md"), "{:?}", files.keys());
        for (path, text) in &files {
            let lower = text.to_lowercase();
            for setting in ["attribution", "includecoauthoredby", "coauthoredby"] {
                assert!(
                    !lower.contains(setting) || path == "AGENTS.md",
                    "{path} holds {setting}"
                );
            }
        }
        let agents = &files["AGENTS.md"];
        assert!(
            agents.contains("A commit made by an agent ends with a `Co-Authored-By:` trailer"),
            "{agents}"
        );
    }

    /// Spec §7.12 "Generated Commits rule": the section is the last of `AGENTS.md`, with the
    /// winning client layer's values, and the header names that layer's `layer.yaml` and the
    /// other layer's does not appear as a source.
    #[test]
    fn the_commits_section_is_last_and_names_the_winning_layer_in_the_header() {
        let files = built();
        let agents = &files["AGENTS.md"];
        let (_, commits) = agents
            .split_once("\n## Commits\n")
            .expect("a Commits section");
        assert!(!commits.contains("\n## "), "no section follows Commits");
        assert!(
            commits.contains("`Owner Example <owner@client.example>` (declared by layer `client`)"),
            "{commits}"
        );
        let header = agents.lines().next().expect("a header");
        assert!(header.contains("client@0.1.0:layer.yaml"), "{header}");
        assert!(!header.contains("personal@1.0.0:layer.yaml"), "{header}");
    }
}
