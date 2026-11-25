//! Spec §13 item 16, "Hub", end to end on the fixture `16-hub` with a hub that is a git
//! repository tagged `layer-v0.1.0` (TASK-55.9.9): both repositories build, each grants the hub
//! as an additional directory, a documentation commit in the hub after the tag changes neither
//! repository's build, and `alz doctor` warns about the `CLAUDE.md` at the client root. Every run
//! has its own home and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// The fixture copied to `work/`, its hub a git repository with the tag `layer-v0.1.0`.
    struct Client {
        scratch: ScratchDir,
    }

    impl Client {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/16-hub");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            scratch.make_dirs("home").expect("made");
            scratch.make_dirs("cache").expect("made");
            let client = Self { scratch };
            client.git(&["init", "--quiet", "--initial-branch=main"]);
            client.git(&["add", "."]);
            client.git(&["commit", "--quiet", "-m", "hub: client layer and docs"]);
            client.git(&["tag", "layer-v0.1.0"]);
            client
        }

        fn git(&self, arguments: &[&str]) {
            let output = Command::new("git")
                .current_dir(self.scratch.join("work/hub"))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "test")
                .env("GIT_AUTHOR_EMAIL", "test@example.com")
                .env("GIT_COMMITTER_NAME", "test")
                .env("GIT_COMMITTER_EMAIL", "test@example.com")
                .args(arguments)
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {arguments:?}");
        }

        fn alz(&self, repository: &str, command: &str) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(self.scratch.join("work").join(repository))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .arg(command)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }
    }

    /// Spec §13 item 16: `path:../hub/layer@layer-v0.1.0` resolves, and each repository's output
    /// grants the hub as an additional directory.
    #[test]
    fn each_repository_of_the_hub_fixture_builds_and_grants_the_hub_as_an_additional_directory() {
        let client = Client::new();
        for repository in ["repo-a", "repo-b"] {
            let run = client.alz(repository, "build");
            assert_eq!(run.code, 0, "{repository}: {}{}", run.stdout, run.stderr);
            let settings = client
                .scratch
                .read_text(format!("work/{repository}/.claude/settings.json"))
                .expect("settings.json is written");
            assert!(
                settings.contains("\"additionalDirectories\": [\n      \"../hub\"\n    ]"),
                "{repository}: {settings}"
            );
        }
    }

    /// Spec §13 item 16: a documentation commit in the hub after the tag changes neither
    /// repository's build: every path, byte and mode of both repositories is as before, and
    /// `alz check` still passes.
    #[test]
    fn a_docs_commit_in_the_hub_after_the_tag_changes_neither_repositorys_build() {
        let client = Client::new();
        for repository in ["repo-a", "repo-b"] {
            assert_eq!(client.alz(repository, "build").code, 0, "{repository}");
        }
        let before = ["repo-a", "repo-b"].map(|repository| {
            read_tree(&client.scratch.join("work").join(repository)).expect("read")
        });
        client
            .scratch
            .write("work/hub/docs/product/overview.md", "a later docs change\n")
            .expect("written");
        client.git(&["add", "."]);
        client.git(&["commit", "--quiet", "-m", "docs: after the tag"]);
        for (repository, tree) in ["repo-a", "repo-b"].into_iter().zip(before) {
            let rebuilt = client.alz(repository, "build");
            assert_eq!(rebuilt.code, 0, "{repository}: {}", rebuilt.stderr);
            assert_eq!(
                read_tree(&client.scratch.join("work").join(repository)).expect("read"),
                tree,
                "{repository}: the build changed"
            );
            let checked = client.alz(repository, "check");
            assert_eq!(
                checked.code, 0,
                "{repository}: {}{}",
                checked.stdout, checked.stderr
            );
        }
    }

    /// Spec §13 item 16: `alz doctor` warns about a `CLAUDE.md` at the client root, with the hub a
    /// git repository and the build clean.
    #[test]
    fn doctor_warns_about_the_client_root_claude_md_with_a_git_hub_and_a_clean_build() {
        let client = Client::new();
        let root = client.scratch.real_path().expect("real path").join("work");
        let run = client.alz("repo-a", "doctor");
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout.contains(&format!(
                "warning: client root {}/CLAUDE.md exists; remove it (put client content in the hub layer)",
                root.display()
            )),
            "{}",
            run.stdout
        );
    }
}
