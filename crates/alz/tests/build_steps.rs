//! `alz build` through the binary against the steps of spec §6 "Steps 3 to 7": a failed check of
//! step 5 (slots filled, budgets met, leak guard on public layers, permission floor kept) stops
//! the build with exit 1 and leaves the repository, and every layer beside it, byte-identical;
//! step 6 renders without touching the repository (nothing is written before the render and the
//! checks have succeeded), and step 7 writes only through a temporary file in the folder of each
//! generated path, so a write that fails part way leaves no generated file that is half written,
//! no temporary entry and no lock. The parts are proved by the merge, render and check tests of
//! §7 to §9 named in `docs/spec-coverage.md`. Every run has its own `HOME` and `XDG_*` folders.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree, read_tree, running_as_root};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name)
    }

    /// An empty scratch directory with a home and the three XDG folders of its own.
    fn scratch() -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["home", "config", "state", "cache", "work"] {
            scratch.make_dirs(folder).expect("made");
        }
        scratch
    }

    /// `alz build` in `work/<repository>`.
    fn build(scratch: &ScratchDir, repository: &str) -> Run {
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(scratch.join("work").join(repository))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .timeout(Duration::from_secs(60))
            .arg("build")
            .output()
            .expect("alz runs and ends in time");
        Run {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// The build fails with exit 1, the error `code` on standard error, nothing on standard
    /// output, and every file below `work` (the repository and its layers, bytes and modes) as it
    /// was.
    fn assert_stops_and_leaves_everything_as_it_was(
        scratch: &ScratchDir,
        repository: &str,
        code: &str,
    ) {
        let before = read_tree(&scratch.join("work")).expect("read");
        let run = build(scratch, repository);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""), "{}", run.stderr);
        assert!(
            run.stderr
                .lines()
                .any(|line| line.starts_with(&format!("error [{code}] "))),
            "{}",
            run.stderr
        );
        assert_eq!(read_tree(&scratch.join("work")).expect("read"), before);
    }

    /// Spec §6 step 5, "slots filled": a layer requires a slot, nothing fills it and
    /// `strict_slots` is on, so the build stops and writes nothing.
    #[test]
    fn a_missing_required_slot_with_strict_slots_stops_the_build_and_writes_nothing() {
        let scratch = scratch();
        copy_tree(&fixture("06-slots"), &scratch.join("work")).expect("copied");
        assert_stops_and_leaves_everything_as_it_was(&scratch, "repo-strict", "slot-missing");
    }

    /// Spec §6 step 5, "budgets met": the instructions are longer than
    /// `budgets.instructions_max_lines`, so the build stops and writes nothing.
    #[test]
    fn an_instruction_budget_that_is_exceeded_stops_the_build_and_writes_nothing() {
        let scratch = scratch();
        for (path, text) in [
            (
                "work/repo/layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\nbudgets:\n  instructions_max_lines: 1\n",
            ),
            (
                "work/repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
            (
                "work/repo/.layer/instructions/00-core.md",
                "one\ntwo\nthree\nfour\n",
            ),
        ] {
            scratch
                .make_dirs(Path::new(path).parent().expect("a folder"))
                .expect("made");
            scratch.write(path, text).expect("written");
        }
        assert_stops_and_leaves_everything_as_it_was(&scratch, "repo", "budget");
    }

    /// Spec §6 step 5, "leak guard on public layers": a public layer holds text that its own
    /// `forbid_patterns` match, so the build stops and writes nothing.
    #[test]
    fn a_leak_guard_match_in_a_public_layer_stops_the_build_and_writes_nothing() {
        let scratch = scratch();
        copy_tree(
            &fixture("10-leak-guard/layer-forbidden"),
            &scratch.join("work/personal"),
        )
        .expect("copied");
        for (path, text) in [
            (
                "work/personal/instructions/00-core.md",
                "- Ask the team at Acme first.\n",
            ),
            (
                "work/repo/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            (
                "work/repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
        ] {
            scratch
                .make_dirs(Path::new(path).parent().expect("a folder"))
                .expect("made");
            scratch.write(path, text).expect("written");
        }
        assert_stops_and_leaves_everything_as_it_was(&scratch, "repo", "leak-forbid");
    }

    /// Spec §6 step 5, "permission floor kept": a more specific layer removes a deny rule of a
    /// general one, so the build stops and writes nothing.
    #[test]
    fn a_permission_floor_that_is_loosened_stops_the_build_and_writes_nothing() {
        let scratch = scratch();
        copy_tree(&fixture("07-permission-floor"), &scratch.join("work")).expect("copied");
        assert_stops_and_leaves_everything_as_it_was(&scratch, "repo-remove", "permission-floor");
    }

    /// Spec §6 steps 6 and 7 with §10.3 "Writing": a build whose write fails part way (a folder
    /// that cannot be written) ends with the error `write`, exit 1, no lock, no temporary entry
    /// (`.alz-<pid>-<n>.tmp`) anywhere in the repository, and no generated file below the folder
    /// that failed; a file that was written before is complete (the whole generated text).
    #[test]
    fn a_write_that_fails_part_way_leaves_no_lock_no_temporary_entry_and_no_half_written_file() {
        if running_as_root() {
            return; // root writes into a folder of mode 0555
        }
        let scratch = scratch();
        for (path, text) in [
            (
                "work/repo/layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            (
                "work/repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
            (
                "work/repo/.layer/agents/a.md",
                "---\nname: a\ndescription: d\n---\nBody.\n",
            ),
        ] {
            scratch
                .make_dirs(Path::new(path).parent().expect("a folder"))
                .expect("made");
            scratch.write(path, text).expect("written");
        }
        scratch.make_dirs("work/repo/.claude/agents").expect("made");
        scratch
            .set_mode("work/repo/.claude/agents", 0o555)
            .expect("made read-only");

        let run = build(&scratch, "repo");

        scratch
            .set_mode("work/repo/.claude/agents", 0o755)
            .expect("restored");
        assert_eq!((run.code, run.stdout.as_str()), (1, ""), "{}", run.stderr);
        assert!(
            run.stderr
                .lines()
                .any(|line| line.starts_with("error [write] ")),
            "{}",
            run.stderr
        );
        let after = read_tree(&scratch.join("work/repo")).expect("read");
        assert!(!after.contains_key("layer.lock"), "{:?}", after.keys());
        assert!(
            after.keys().all(|path| !path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.starts_with(".alz-")
                    && Path::new(name).extension().is_some_and(|ext| ext == "tmp"))),
            "a temporary entry was left: {:?}",
            after.keys()
        );
        assert!(!after.contains_key(".claude/agents/a.md"));
        // The folder can be written again: the next build succeeds, and each file the failed run
        // had written is exactly the file the successful build writes (none was half written).
        let again = build(&scratch, "repo");
        assert_eq!(again.code, 0, "{}", again.stderr);
        let rebuilt = read_tree(&scratch.join("work/repo")).expect("read");
        assert!(
            after
                .iter()
                .all(|(path, file)| rebuilt.get(path) == Some(file)),
            "{:?}",
            after.keys()
        );
    }
}
