//! Spec §13 items 12 and 24 through `alz build` (TASK-55.9.9). Item 12, "Account independence":
//! the build, whose effective agents and skills are its output, does not depend on the account
//! folder `CLAUDE_CONFIG_DIR` names. Item 24, "Untrusted repository", account cases: a layer that
//! may not declare `account` and does, fails the build, names the layers and writes nothing, while
//! the one allowed client layer builds. Fixture `29-account-layer`; every run has an isolated
//! home, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, TreeFile, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stderr: String,
    }

    /// A scratch copy of `fixtures/29-account-layer` with the manifest of the repository
    /// replaced, and `account: client-a` added to the layers named.
    struct World {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl World {
        fn new(stack: &[&str], declaring: &[&str]) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/29-account-layer");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            let entries: String = stack
                .iter()
                .map(|source| ["  - source: path:", source, "\n"].concat())
                .collect();
            scratch
                .write(
                    "work/repo/layer.yaml",
                    format!("stack:\n{entries}targets: [claude]\n"),
                )
                .expect("written");
            for layer in declaring {
                let file = if *layer == "repo" {
                    "work/repo/.layer/layer.yaml".to_owned()
                } else {
                    format!("work/{layer}/layer.yaml")
                };
                let text = scratch.read_text(&file).expect("read");
                scratch
                    .write(&file, format!("{text}account: client-a\n"))
                    .expect("written");
            }
            for folder in ["home", "cache", "state", "config"] {
                scratch.make_dirs(folder).expect("made");
            }
            Self {
                repository: scratch.join("work/repo"),
                scratch,
            }
        }

        fn alz(&self, arguments: &[&str], account_folder: Option<&str>) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            command
                .current_dir(&self.repository)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(arguments);
            if let Some(folder) = account_folder {
                command.env("CLAUDE_CONFIG_DIR", self.scratch.join(folder));
            }
            let output = command.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn tree(&self) -> BTreeMap<String, TreeFile> {
            read_tree(&self.repository).expect("the repository is read")
        }
    }

    const CLIENT_STACK: [&str; 3] = ["../personal", "../client", "./.layer"];

    /// Spec §13 item 12: with `CLAUDE_CONFIG_DIR` at two different empty account folders, the
    /// build writes the same agents, skills and every other file, and `alz check` passes under
    /// the other folder.
    #[test]
    fn the_build_is_the_same_under_two_different_empty_account_folders() {
        let first = World::new(&CLIENT_STACK, &[]);
        first.scratch.make_dirs("account-one").expect("made");
        let second = World::new(&CLIENT_STACK, &[]);
        second.scratch.make_dirs("account-two").expect("made");
        assert_eq!(first.alz(&["build"], Some("account-one")).code, 0);
        assert_eq!(second.alz(&["build"], Some("account-two")).code, 0);
        let (one, two) = (first.tree(), second.tree());
        assert!(one.keys().any(|path| path == "AGENTS.md"), "{one:?}");
        assert_eq!(one, two);
        assert_eq!(first.alz(&["check"], Some("account-two")).code, 0);
    }

    /// Spec §13 item 24: each declarer that may not declare fails the build with the layers
    /// named, and nothing is written.
    #[test]
    fn a_layer_that_may_not_declare_an_account_fails_the_build_and_names_it() {
        let cases: [(&str, &[&str], &[&str]); 4] = [
            ("a repo layer", &CLIENT_STACK, &["repo"]),
            (
                "a group layer",
                &["../personal", "../client", "../group", "./.layer"],
                &["group"],
            ),
            (
                "a public client layer",
                &["../personal", "../public-client", "./.layer"],
                &["public-client"],
            ),
            (
                "a second client layer",
                &["../personal", "../client", "../client-b", "./.layer"],
                &["client", "client-b"],
            ),
        ];
        for (what, stack, declaring) in cases {
            let world = World::new(stack, declaring);
            let before = world.tree();
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{what}: {}", run.stderr);
            let line = run
                .stderr
                .lines()
                .find(|line| line.starts_with("error [account]"))
                .unwrap_or_else(|| panic!("{what}: no error [account]: {}", run.stderr));
            for name in declaring {
                assert!(line.contains(name), "{what}: {line}");
            }
            assert_eq!(world.tree(), before, "{what}: nothing is written");
        }
    }

    /// Spec §13 item 24: the one allowed client layer outside the repository declares an
    /// account and the build succeeds.
    #[test]
    fn the_private_client_layer_outside_the_repository_may_declare_an_account() {
        let world = World::new(&CLIENT_STACK, &["client"]);
        let run = world.alz(&["build"], None);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(!run.stderr.contains("[account]"), "{}", run.stderr);
    }
}
