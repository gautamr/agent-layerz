//! `alz build` and `alz check` against the reference's output (spec §4.3, §10.3, TASK-55.6.8):
//! for every repository fixture of the acceptance suite that builds, the Rust `alz` writes the
//! paths, bytes and executable bits of the golden tree made from the TypeScript reference by
//! `crates/alz/tools/golden-trees.mjs`, prints what it printed, and `alz check` then passes. A
//! generated file that is edited makes `alz check` fail, as in the reference.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, TreeFile, copy_tree, folder_names, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    /// The fixtures of the acceptance suite, which the golden trees were made from.
    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    fn goldens() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden")
    }

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// The level and code of each line of diagnostics, in order. The place and the wording after
    /// them are the Rust `alz`'s own where the specification words a message differently from the
    /// reference (§7.3 `slot-missing` has a place, §9.2 reports a finding once per file); the
    /// messages are tested where they are made. The `[identity]` lines of §9.4 are compared like
    /// the others: a fixture with a declared commit identity, built outside a git repository,
    /// prints the note that it was not checked.
    fn problem_lines(text: &str) -> Vec<String> {
        text.lines()
            .map(|line| {
                line.split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    struct Case {
        fixture: String,
        repo: String,
    }

    impl Case {
        fn name(&self) -> String {
            format!("{}/{}", self.fixture, self.repo)
        }

        fn golden(&self) -> PathBuf {
            goldens().join(&self.fixture).join(&self.repo)
        }
    }

    fn cases() -> Vec<Case> {
        folder_names(&goldens())
            .into_iter()
            .flat_map(|fixture| {
                folder_names(&goldens().join(&fixture))
                    .into_iter()
                    .map(move |repo| Case {
                        fixture: fixture.clone(),
                        repo,
                    })
            })
            .collect()
    }

    fn git(directory: &Path, home: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .current_dir(directory)
            .env("HOME", home)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
            .args(arguments)
            .output()
            .expect("git runs")
            .status;
        assert!(status.success(), "git {arguments:?}");
    }

    /// A scratch copy of the case's fixture, with its own home and cache.
    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn of(case: &Case) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            copy_tree(&fixtures().join(&case.fixture), &work).expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            if case.fixture == "16-hub" {
                let hub = work.join("hub");
                let home = scratch.join("home");
                git(&hub, &home, &["init", "--quiet", "--initial-branch=main"]);
                git(&hub, &home, &["add", "."]);
                git(
                    &hub,
                    &home,
                    &["commit", "--quiet", "-m", "hub: client layer and docs"],
                );
                git(&hub, &home, &["tag", "layer-v0.1.0"]);
            }
            let repository = work.join(&case.repo);
            Self {
                scratch,
                repository,
            }
        }

        fn alz(&self, command: &str) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
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

    /// What differs between the golden tree and what the build made, as lines.
    fn differences(
        expected: &BTreeMap<String, TreeFile>,
        made: &BTreeMap<String, TreeFile>,
    ) -> Vec<String> {
        let missing = expected
            .keys()
            .filter(|path| !made.contains_key(*path))
            .map(|path| format!("not written: {path}"));
        let unexpected = made
            .keys()
            .filter(|path| !expected.contains_key(*path))
            .map(|path| format!("not in the golden tree: {path}"));
        let changed = expected.iter().filter_map(|(path, file)| {
            made.get(path).and_then(|found| {
                if found.bytes != file.bytes {
                    Some(format!("bytes differ: {path}"))
                } else if found.executable != file.executable {
                    Some(format!("executable bit differs: {path}"))
                } else {
                    None
                }
            })
        });
        missing.chain(unexpected).chain(changed).collect()
    }

    fn text_of(golden: &BTreeMap<String, TreeFile>, name: &str) -> String {
        golden.get(name).map_or_else(String::new, |file| {
            String::from_utf8_lossy(&file.bytes).into_owned()
        })
    }

    /// Builds the case, compares what was written, runs `alz check`, edits a generated file and
    /// runs it again. The problems found, as lines.
    fn problems_of(case: &Case) -> Vec<String> {
        let workspace = Workspace::of(case);
        let before = read_tree(&workspace.repository).expect("the repository is read");
        let build = workspace.alz("build");
        if build.code != 0 {
            return vec![format!("build exited {}: {}", build.code, build.stderr)];
        }
        let made: BTreeMap<String, TreeFile> = read_tree(&workspace.repository)
            .expect("the repository is read")
            .into_iter()
            .filter(|(path, file)| before.get(path) != Some(file))
            .collect();
        let golden = read_tree(&case.golden()).expect("the golden tree is read");
        let expected: BTreeMap<String, TreeFile> = golden
            .iter()
            .filter_map(|(path, file)| {
                path.strip_prefix("files/")
                    .map(|path| (path.to_owned(), file.clone()))
            })
            .collect();
        let stdout = (build.stdout != text_of(&golden, "stdout"))
            .then(|| format!("stdout differs: {}", build.stdout));
        let stderr = (problem_lines(&build.stderr) != problem_lines(&text_of(&golden, "stderr")))
            .then(|| format!("stderr differs: {}", build.stderr));
        let check = workspace.alz("check");
        let checked =
            (check.code != 0).then(|| format!("check exited {}: {}", check.code, check.stderr));
        let edited = edit_is_caught(&workspace);
        differences(&expected, &made)
            .into_iter()
            .chain(stdout)
            .chain(stderr)
            .chain(checked)
            .chain(edited)
            .collect()
    }

    /// After an edit of `AGENTS.md`, `alz check` fails and says which file differs.
    fn edit_is_caught(workspace: &Workspace) -> Option<String> {
        let path = workspace.repository.join("AGENTS.md");
        let text = workspace
            .scratch
            .read(path.strip_prefix(workspace.scratch.path()).ok()?);
        let mut edited = text.ok()?;
        edited.extend_from_slice(b"hand edit\n");
        workspace
            .scratch
            .write(path.strip_prefix(workspace.scratch.path()).ok()?, edited)
            .ok()?;
        let check = workspace.alz("check");
        let expected = [
            "error [check] AGENTS.md: differs from a fresh build",
            "check failed: run `alz build` and commit the result",
        ];
        let verdict: Vec<&str> = check
            .stderr
            .lines()
            .filter(|line| line.starts_with("error [check]") || line.starts_with("check failed"))
            .collect();
        (check.code != 1 || verdict != expected).then(|| {
            format!(
                "an edited file: check exited {}: {}",
                check.code, check.stderr
            )
        })
    }

    #[test]
    fn every_golden_tree_is_what_build_writes_and_check_accepts() {
        let cases = cases();
        assert!(cases.len() >= 20, "the golden trees are found");
        let problems: Vec<String> = cases
            .iter()
            .flat_map(|case| {
                problems_of(case)
                    .into_iter()
                    .map(|problem| format!("{}: {problem}", case.name()))
            })
            .collect();
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
