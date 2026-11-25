//! Cache integrity (spec §5.3 "Cache integrity", §6 step 1; fixture `28-cache-integrity`,
//! TASK-55.9.6): a layer read from the layer cache is checked against `layer.lock`, and every
//! command that resolves the stack stops, with the error `cache` and no file content, when the
//! entry was edited after it was written. The sources are real `git` repositories made in a
//! scratch directory; `HOME` and `XDG_CACHE_HOME` are isolated.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_io::EnvSnapshot;
    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const MARKER: &str = "TAMPER-MARKER-7f3a";

    fn real_path() -> std::ffi::OsString {
        EnvSnapshot::capture()
            .vars()
            .get("PATH")
            .expect("PATH is set")
            .to_owned()
    }

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct World {
        scratch: ScratchDir,
        root: PathBuf,
    }

    impl World {
        /// The fixture copied into a scratch directory, its `layer-a` made a repository on
        /// `main`, and the manifest of the repository naming that repository as its first layer.
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/28-cache-integrity"),
                &root.join("work"),
            )
            .expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            let world = Self { scratch, root };
            let layer = world.root.join("work/layer-a");
            world.vcs(&layer, &["init", "-q"]);
            world.vcs(&layer, &["add", "-A"]);
            world.vcs(&layer, &["commit", "-q", "-m", "layer a"]);
            world
                .scratch
                .write(
                    "work/repo/layer.yaml",
                    format!(
                        "targets: [claude]\nstack:\n  - source: {}\n  - source: path:./.layer\n",
                        world.source()
                    ),
                )
                .expect("a manifest");
            world
        }

        fn source(&self) -> String {
            format!(
                "git:file://{}@main",
                self.root.join("work/layer-a").display()
            )
        }

        fn vcs(&self, directory: &Path, args: &[&str]) {
            let output = Command::new("git")
                .current_dir(directory)
                .env_clear()
                .env("PATH", real_path())
                .env("HOME", self.root.join("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args([
                    "-c",
                    "init.defaultBranch=main",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .expect("the version control tool runs");
            assert!(output.status.success(), "{args:?}: {output:?}");
        }

        fn alz(&self, args: &[&str]) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            let output = command
                .current_dir(self.root.join("work/repo"))
                .env_clear()
                .env("PATH", real_path())
                .env("HOME", self.root.join("home"))
                .env("XDG_CACHE_HOME", self.root.join("xdg"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", self.root.join("gitconfig"))
                .timeout(Duration::from_secs(60))
                .args(args)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        /// The one entry of the layer cache, as a path relative to the scratch directory.
        fn entry(&self) -> String {
            let names = self.scratch.list("xdg/agent-layerz");
            assert_eq!(names.len(), 1, "{names:?}");
            format!("xdg/agent-layerz/{}", names[0])
        }
    }

    /// Spec §5.3 "Cache integrity": after a build has fetched the layer and written the lock, an
    /// entry that a file was added to is the error `cache` for every command that resolves the
    /// stack. The message names the lock entry, the source and the entry's directory, says to
    /// delete it, and holds no file content. `doctor` reports it in its own report on standard
    /// output.
    #[test]
    fn every_command_that_resolves_the_stack_refuses_a_cache_entry_that_changed_after_it_was_cached()
     {
        let world = World::new();
        let built = world.alz(&["build"]);
        assert_eq!(
            (built.code, built.stderr.as_str()),
            (0, ""),
            "{}",
            built.stdout
        );
        let entry = world.entry();
        world
            .scratch
            .write(format!("{entry}/extra.md"), MARKER)
            .expect("tampered");
        let message = format!(
            "error [cache] cached layer 'team-a' ({}) no longer matches layer.lock: its files in {} changed after they were cached; delete that directory and rebuild\n",
            world.source(),
            world.root.join(&entry).display()
        );

        for command in [
            &["build"][..],
            &["check"],
            &["diff"],
            &["doctor"],
            &["ls"],
            &["explain", "instruction", "10-team"],
            &["lint"],
        ] {
            let run = world.alz(command);
            assert_eq!(run.code, 1, "{command:?}: {}", run.stderr);
            assert!(
                format!("{}{}", run.stderr, run.stdout).contains(message.trim_end()),
                "{command:?}: {}\nstdout: {}\nexpected {message}",
                run.stderr,
                run.stdout
            );
            assert!(
                !run.stdout.contains(MARKER) && !run.stderr.contains(MARKER),
                "{command:?} printed file content"
            );
        }
    }

    /// Spec §5.3 "Cache integrity" and §6 step 1: nothing is compared with the lock when the
    /// lock has no entry for the source; the existing git entry is then compared with a fresh
    /// clone of the same commit, and the difference is the other cache error.
    #[test]
    fn a_source_the_lock_does_not_list_is_compared_with_a_fresh_clone_and_not_with_the_lock() {
        let world = World::new();
        assert_eq!(world.alz(&["build"]).code, 0);
        let entry = world.entry();
        world
            .scratch
            .write(format!("{entry}/extra.md"), MARKER)
            .expect("tampered");
        world
            .scratch
            .remove_file("work/repo/layer.lock")
            .expect("no lock");

        let run = world.alz(&["lint"]);

        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            !run.stderr.contains("no longer matches layer.lock"),
            "{}",
            run.stderr
        );
        assert!(
            run.stderr.contains(&format!(
                "error [cache] cached layer '{}' differs from a fresh clone of the same commit: its files in {} changed after they were cached; delete that directory and rebuild\n",
                world.source(),
                world.root.join(&entry).display()
            )),
            "{}",
            run.stderr
        );
        assert!(!run.stderr.contains(MARKER));
    }
}
