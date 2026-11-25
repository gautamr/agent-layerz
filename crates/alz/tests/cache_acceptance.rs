//! Spec §13 item 28, "Sources and the layer cache through `alz lint`" (TASK-55.9.9): the same
//! verdicts through `alz build`, the cache folder when `XDG_CACHE_HOME` is empty or relative, a
//! layer reached through the home or a link into an account folder, the pre-commit hook case in a
//! main checkout and in a linked worktree for `alz lint` and `alz check`, and the name with the
//! byte 0xFF. Every run has its own home, cache and git configuration; the layers are local git
//! repositories and folders, so nothing leaves the machine.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const LAYER: &str = "name: team\nkind: group\nvisibility: private\nversion: 1.0.0\n";
    const REPO_LAYER: &str = "name: shop-api\nkind: repo\nvisibility: private\n";

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
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            for name in ["home", "repo/.layer"] {
                scratch.make_dirs(name).expect("made");
            }
            scratch
                .write("repo/.layer/layer.yaml", REPO_LAYER)
                .expect("written");
            Self { scratch, root }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn put(&self, relative: &str, text: &str) {
            if let Some(parent) = Path::new(relative).parent() {
                self.scratch.make_dirs(parent).expect("made");
            }
            self.scratch.write(relative, text).expect("written");
        }

        fn manifest(&self, first_source: &str) {
            self.put(
                "repo/layer.yaml",
                &format!(
                    "stack:\n  - source: {first_source}\n  - source: path:./.layer\ntargets: [claude]\n"
                ),
            );
        }

        fn command(&self, directory: &Path, cache: Option<&str>) -> Command {
            let mut command: Command = cargo_bin_cmd!("alz");
            command
                .current_dir(directory)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.path("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .timeout(Duration::from_secs(60));
            if let Some(cache) = cache {
                command.env("XDG_CACHE_HOME", cache);
            }
            command
        }

        fn alz(&self, arguments: &[&str], cache: Option<&str>) -> Run {
            let output = self
                .command(&self.path("repo"), cache)
                .args(arguments)
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn git(&self, directory: &Path, arguments: &[&str]) {
            let status = Command::new("git")
                .current_dir(directory)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.path("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args([
                    "-c",
                    "init.defaultBranch=main",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(arguments)
                .output()
                .expect("git runs")
                .status;
            assert!(status.success(), "git {arguments:?}");
        }

        /// A local git repository holding the layer, with the commit on `main`.
        fn layer_repository(&self) {
            self.put("work/layer.yaml", LAYER);
            self.git(&self.path("work"), &["init", "-q"]);
            self.git(&self.path("work"), &["add", "-A"]);
            self.git(&self.path("work"), &["commit", "-q", "-m", "layer"]);
        }

        fn git_source(&self) -> String {
            format!("git:file://{}@main", self.path("work").display())
        }
    }

    /// Spec §13 item 28: a git layer gives the same verdict through `alz lint` and `alz build`,
    /// and both fill the cache.
    #[test]
    fn a_git_layer_is_resolved_alike_by_lint_and_build() {
        for command in ["lint", "build"] {
            let world = World::new();
            world.layer_repository();
            world.manifest(&world.git_source());
            let run = world.alz(&[command], Some(world.path("xdg").to_str().expect("text")));
            assert_eq!(run.code, 0, "{command}: {}", run.stderr);
            assert!(!run.stderr.contains("error"), "{command}: {}", run.stderr);
            assert_eq!(
                world.scratch.list("xdg/agent-layerz").len(),
                1,
                "{command}: one cache entry"
            );
        }
    }

    /// Spec §13 item 28: a `git:` source with credentials is the error `source` through `alz
    /// lint` and `alz build`, and the token appears nowhere in the output.
    #[test]
    fn a_source_with_credentials_is_the_error_source_through_lint_and_build() {
        for command in ["lint", "build"] {
            let world = World::new();
            world.manifest("git:https://user:s3cr3t-token@example.invalid/team/layer.git@main");
            let run = world.alz(&[command], Some(world.path("xdg").to_str().expect("text")));
            assert_eq!(run.code, 1, "{command}");
            assert!(run.stderr.contains("error [source]"), "{}", run.stderr);
            assert!(
                !(run.stderr.contains("s3cr3t-token") || run.stdout.contains("s3cr3t-token")),
                "{command}: the token is printed"
            );
            assert!(!world.scratch.exists("xdg"), "{command}: no cache is made");
        }
    }

    /// Spec §13 item 28: a failed fetch prints `cannot fetch '<source>': <line>` through `alz
    /// lint` and `alz build`.
    #[test]
    fn a_failed_fetch_is_cannot_fetch_through_lint_and_build() {
        for command in ["lint", "build"] {
            let world = World::new();
            let source = format!("git:file://{}@main", world.path("missing").display());
            world.manifest(&source);
            let run = world.alz(&[command], Some(world.path("xdg").to_str().expect("text")));
            assert_eq!(run.code, 1, "{command}");
            assert!(
                run.stderr.contains(&format!("cannot fetch '{source}': ")),
                "{command}: {}",
                run.stderr
            );
        }
    }

    /// Spec §13 item 28: with `XDG_CACHE_HOME` empty or relative the cache is
    /// `~/.cache/agent-layerz` under the home, and the repository tree is unchanged.
    #[test]
    fn an_empty_or_relative_xdg_cache_home_puts_the_cache_under_the_home() {
        for cache in ["", "relative-cache"] {
            let world = World::new();
            world.layer_repository();
            world.manifest(&world.git_source());
            let before = read_tree(&world.path("repo")).expect("read");
            let run = world.alz(&["lint"], Some(cache));
            assert_eq!(run.code, 0, "{cache:?}: {}", run.stderr);
            assert_eq!(
                world.scratch.list("home/.cache/agent-layerz").len(),
                1,
                "{cache:?}"
            );
            assert!(!world.scratch.exists("repo/relative-cache"));
            assert_eq!(read_tree(&world.path("repo")).expect("read"), before);
        }
    }

    /// Spec §13 item 28: a `path:` layer at the home, or reached through a link into an account
    /// folder, is refused through `alz lint` and `alz build` before anything in it is opened.
    #[test]
    fn a_layer_at_the_home_or_behind_a_link_into_an_account_folder_is_refused_by_lint_and_build() {
        for command in ["lint", "build"] {
            let at_home = World::new();
            at_home.put("home/layer.yaml", LAYER);
            at_home.manifest("path:../home");
            let run = at_home.alz(&[command], None);
            assert_eq!(run.code, 1, "{command}: {}", run.stderr);
            assert!(run.stderr.contains("error ["), "{}", run.stderr);

            let linked = World::new();
            linked.put("home/.claude-x/layers/team/layer.yaml", LAYER);
            linked
                .scratch
                .make_link("link", linked.path("home/.claude-x/layers/team"))
                .expect("a link");
            linked.manifest("path:../link");
            let run = linked.alz(&[command], None);
            assert_eq!(run.code, 1, "{command}: {}", run.stderr);
            assert!(run.stderr.contains("error ["), "{}", run.stderr);
        }
    }

    /// A repository with a committed layer and manifest, ready for a hook: the hook runs `alz
    /// <command>` and compares the index it sees before and after.
    fn hooked_checkout(world: &World, command: &str) {
        world.manifest("path:./.layer");
        world.put(
            "repo/layer.yaml",
            "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
        );
        world.put("repo/.layer/instructions/00-core.md", "- standard\n");
        world.git(&world.path("repo"), &["init", "-q"]);
        if command == "check" {
            let built = world.alz(&["build"], None);
            assert_eq!(built.code, 0, "{}", built.stderr);
        }
        world.git(&world.path("repo"), &["add", "-A"]);
        world.git(
            &world.path("repo"),
            &["commit", "-q", "-m", "first", "--no-verify"],
        );
        let alz = assert_cmd::cargo::cargo_bin!("alz");
        let hook = format!(
            "#!/bin/sh\nindex=\"${{GIT_INDEX_FILE:-$(git rev-parse --git-path index)}}\"\ncp \"$index\" '{saved}'\n'{alz}' {command} || exit 1\ncmp \"$index\" '{saved}' || exit 1\n",
            saved = world.path("index-before").display(),
            alz = alz.display(),
        );
        world.scratch.make_dirs("repo/.git/hooks").expect("made");
        world
            .scratch
            .write_executable("repo/.git/hooks/pre-commit", hook)
            .expect("a hook");
    }

    fn hook_leaves_index_and_tree_alone(command: &str, linked: bool) {
        let world = World::new();
        hooked_checkout(&world, command);
        let directory = if linked {
            world.git(
                &world.path("repo"),
                &["worktree", "add", "-q", "../worktree", "-b", "other"],
            );
            world.path("worktree")
        } else {
            world.path("repo")
        };
        world.put(
            &format!("{}/change.txt", if linked { "worktree" } else { "repo" }),
            "a change\n",
        );
        world.git(&directory, &["add", "change.txt"]);
        let before = read_tree(&directory).expect("read");
        let status = Command::new("git")
            .current_dir(&directory)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", world.path("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(["-c", "commit.gpgsign=false", "commit", "-q", "-m", "change"])
            .output()
            .expect("git runs");
        assert!(
            status.status.success(),
            "the hook ran `alz {command}` and it failed or changed the index (linked: {linked}): {}{}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(world.scratch.exists("index-before"), "the hook ran");
        let after = read_tree(&directory).expect("read");
        let without_git =
            |tree: &std::collections::BTreeMap<String, alz_test_support::TreeFile>| {
                tree.iter()
                    .filter(|(path, _)| path.as_str() != ".git" && !path.starts_with(".git/"))
                    .map(|(path, file)| (path.clone(), file.clone()))
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
        assert_eq!(without_git(&after), without_git(&before));
    }

    /// Spec §13 item 28: run as a pre-commit hook in a main checkout, `alz lint` leaves the index
    /// and the working tree byte-identical.
    #[test]
    fn lint_as_a_pre_commit_hook_in_a_main_checkout_leaves_the_index_and_the_tree_alone() {
        hook_leaves_index_and_tree_alone("lint", false);
    }

    /// Spec §13 item 28: the same in a linked worktree.
    #[test]
    fn lint_as_a_pre_commit_hook_in_a_linked_worktree_leaves_the_index_and_the_tree_alone() {
        hook_leaves_index_and_tree_alone("lint", true);
    }

    /// Spec §13 item 28: the pre-commit hook case holds for `alz check` in a main checkout.
    #[test]
    fn check_as_a_pre_commit_hook_in_a_main_checkout_leaves_the_index_and_the_tree_alone() {
        hook_leaves_index_and_tree_alone("check", false);
    }

    /// Spec §13 item 28: and in a linked worktree.
    #[test]
    fn check_as_a_pre_commit_hook_in_a_linked_worktree_leaves_the_index_and_the_tree_alone() {
        hook_leaves_index_and_tree_alone("check", true);
    }

    #[cfg(unix)]
    fn name_with_the_byte_0xff() -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(b"bad-\xff-name.md".to_vec())
    }

    /// Spec §13 item 28: a layer holding a name with the byte 0xFF is the error `layer`, through
    /// `alz lint --layer` and through `alz build`. Only Linux file systems keep such a name, so
    /// the case runs there; the sibling test below shows on every other system that the name
    /// cannot be made, so this case is never skipped without a trace.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_name_with_the_byte_0xff_is_the_error_layer_on_linux() {
        let world = World::new();
        world.put("work/layer.yaml", LAYER);
        world.scratch.make_dirs("work/instructions").expect("made");
        world
            .scratch
            .write(
                Path::new("work/instructions").join(name_with_the_byte_0xff()),
                "text\n",
            )
            .expect("Linux keeps a name with the byte 0xFF");
        world.manifest("path:../work");
        for arguments in [vec!["build"], vec!["lint"]] {
            let run = world.alz(&arguments, None);
            assert_eq!(run.code, 1, "{arguments:?}: {}", run.stderr);
            assert!(run.stderr.contains("error [layer]"), "{}", run.stderr);
        }
    }

    /// Spec §13 item 28: on a file system that refuses the name (macOS), creating it fails, so
    /// the Linux-only case above has nothing to run here and says so.
    #[cfg(all(unix, not(target_os = "linux")))]
    #[test]
    fn a_name_with_the_byte_0xff_cannot_be_made_off_linux_so_the_case_is_linux_only() {
        let world = World::new();
        world.scratch.make_dirs("work").expect("made");
        let made = world
            .scratch
            .write(Path::new("work").join(name_with_the_byte_0xff()), "text\n");
        assert!(
            made.is_err(),
            "this system keeps the name; run the case here too"
        );
    }
}
