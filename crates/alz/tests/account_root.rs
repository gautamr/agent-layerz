//! Which repository `alz account which` resolves for (spec §7.13 "Paths"), against
//! `fixtures/31-account-which`: the nearest directory at or above the working directory whose
//! `layer.yaml` has a `stack` key; a layer's own manifest is skipped, and a nested repository is
//! its own root. Each run has its own `HOME` and no stack is read, because no repository is
//! allowed.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    fn world() -> (ScratchDir, PathBuf) {
        let scratch = ScratchDir::new().expect("a scratch directory");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/31-account-which");
        copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
        scratch.make_dir("home").expect("made");
        let work = scratch
            .real_path()
            .expect("the real path of the scratch folder")
            .join("work");
        (scratch, work)
    }

    fn which_in(scratch: &ScratchDir, directory: &Path) -> Run {
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(directory)
            .env_clear()
            .env("PATH", scratch.join("none"))
            .env("HOME", scratch.join("home"))
            .timeout(Duration::from_secs(60))
            .args(["account", "which"])
            .output()
            .expect("alz runs and ends in time");
        Run {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Spec §7.13 "Paths": from a folder below a repository the root is the nearest manifest with
    /// a `stack`, a nested repository is its own root, and the outer one is not looked at.
    #[test]
    fn the_root_is_the_nearest_manifest_with_a_stack_and_a_nested_repository_is_its_own() {
        let (scratch, work) = world();
        for (directory, root) in [
            ("repo", "repo"),
            ("repo/sub/deeper", "repo"),
            ("repo/nested", "repo/nested"),
        ] {
            let run = which_in(&scratch, &work.join(directory));
            assert_eq!(run.code, 0, "{directory}: {}{}", run.stdout, run.stderr);
            assert!(
                run.stdout
                    .starts_with(&format!("repository: {}\n", work.join(root).display())),
                "{directory}: {}",
                run.stdout
            );
        }
    }

    /// Spec §7.13 "Paths": a `layer.yaml` without a `stack` key, such as a layer's own manifest,
    /// is skipped by the walk, so a layer folder that no repository holds has no repository.
    #[test]
    fn a_layers_own_manifest_is_not_a_repository_root() {
        let (scratch, work) = world();
        let run = which_in(&scratch, &work.join("hub/layer"));
        assert!(
            !run.stdout
                .contains(&format!("repository: {}", work.join("hub/layer").display())),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("no repository") || run.stderr.contains("no repository"),
            "{}{}",
            run.stdout,
            run.stderr
        );
    }
}
