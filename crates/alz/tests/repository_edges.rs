//! Edges of the layer layout and of the repository writer through the binary (spec §4.1 "A layer
//! may contain only `instructions/`", §4.3 "Links", §4.3 "The search for `AGENTS.override.md`" and
//! §4.3 "Writing", TASK-55.9). Every run has a home, a cache and a repository of its own under a
//! scratch directory, and nothing outside it is read or written.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    /// Writing a file with its folders made first, which `ScratchDir::write` does not do.
    trait Put {
        fn put(&self, name: &str, contents: &str) -> std::io::Result<()>;
    }

    impl Put for ScratchDir {
        fn put(&self, name: &str, contents: &str) -> std::io::Result<()> {
            if let Some((folder, _)) = name.rsplit_once('/') {
                self.make_dirs(folder)?;
            }
            self.write(name, contents)
        }
    }

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A repository whose stack is `../personal` and `./.layer`, with a home of its own.
    struct World {
        scratch: ScratchDir,
    }

    impl World {
        /// `personal_files` are the files of the personal layer, besides its `layer.yaml`.
        fn new(personal_files: &[(&str, &str)]) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            scratch.make_dirs("home").expect("made");
            scratch
                .put(
                    "personal/layer.yaml",
                    "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
                )
                .expect("written");
            for (path, text) in personal_files {
                scratch
                    .put(&format!("personal/{path}"), text)
                    .expect("written");
            }
            scratch
                .put(
                    "repo/layer.yaml",
                    "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
                )
                .expect("written");
            scratch
                .put(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
            Self { scratch }
        }

        fn repo(&self) -> PathBuf {
            self.scratch.join("repo")
        }

        fn command(&self) -> Command {
            let mut command: Command = cargo_bin_cmd!("alz");
            command
                .current_dir(self.repo())
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60));
            command
        }

        fn alz(&self, arguments: &[&str]) -> Run {
            let output = self
                .command()
                .args(arguments)
                .output()
                .expect("alz runs and ends in time");
            Run::of(&output)
        }

        /// `alz build` started by a shell that has set its umask first, so the build inherits it.
        fn build_under_umask(&self, umask: &str) -> Run {
            let program = assert_cmd::cargo::cargo_bin("alz");
            let mut command = Command::new("sh");
            let output = command
                .current_dir(self.repo())
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(["-c", &format!("umask {umask} && exec \"$0\" build")])
                .arg(program)
                .output()
                .expect("alz runs and ends in time");
            Run::of(&output)
        }
    }

    impl Run {
        fn of(output: &std::process::Output) -> Self {
            Self {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }
    }

    const AGENT: &str = "---\nname: helper\ndescription: Helps\n---\nYou help.\n";

    /// Spec §4.1: "A layer may contain only `instructions/`". A stack whose personal layer holds a
    /// `layer.yaml` and one instruction builds, and the instruction reaches `AGENTS.md`.
    #[test]
    fn a_layer_that_holds_only_instructions_builds() {
        let world = World::new(&[("instructions/00-core.md", "Keep it small.\n")]);
        let run = world.alz(&["build"]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""), "{}", run.stdout);
        let agents = world
            .scratch
            .read_text("repo/AGENTS.md")
            .expect("AGENTS.md is written");
        assert!(agents.contains("Keep it small."), "{agents}");
    }

    /// Spec §4.3 "Links": a generated path that is a link is the collision `is a link or lies
    /// under one; alz never writes through a link`, exit 1, nothing written and the link's target
    /// untouched.
    #[test]
    fn a_link_at_a_generated_path_is_refused_with_its_message_and_its_target_is_untouched() {
        let world = World::new(&[("instructions/00-core.md", "Keep it small.\n")]);
        world.scratch.put("outside.md", "mine").expect("written");
        world
            .scratch
            .make_link("repo/AGENTS.md", world.scratch.join("outside.md"))
            .expect("linked");
        let before = read_tree(world.scratch.path()).expect("read");
        let run = world.alz(&["build"]);
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (
                1,
                "",
                "error [collision] AGENTS.md: is a link or lies under one; alz never writes through a link\n"
            )
        );
        assert_eq!(read_tree(world.scratch.path()).expect("read"), before);
    }

    /// Spec §4.3 "Links": a generated path that is a folder is the collision `is not a regular
    /// file`, exit 1 and nothing written.
    #[test]
    fn a_folder_at_a_generated_path_is_refused_as_not_a_regular_file() {
        let world = World::new(&[("instructions/00-core.md", "Keep it small.\n")]);
        world.scratch.make_dir("repo/AGENTS.md").expect("made");
        let before = read_tree(world.scratch.path()).expect("read");
        let run = world.alz(&["build"]);
        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (
                1,
                "",
                "error [collision] AGENTS.md: is not a regular file\n"
            )
        );
        assert_eq!(read_tree(world.scratch.path()).expect("read"), before);
    }

    /// Spec §4.3 "The search for `AGENTS.override.md`": the file is a collision wherever it lies,
    /// in a folder that git ignores too, and an entry of that name that is a link is one as well.
    #[test]
    fn an_override_in_an_ignored_folder_or_as_a_link_is_a_collision_and_nothing_is_written() {
        let world = World::new(&[("instructions/00-core.md", "Keep it small.\n")]);
        world
            .scratch
            .put("repo/.gitignore", "build/\n")
            .expect("written");
        world
            .scratch
            .put("repo/build/AGENTS.override.md", "hides")
            .expect("written");
        world.scratch.put("elsewhere.md", "x").expect("written");
        world.scratch.make_dir("repo/docs").expect("made");
        world
            .scratch
            .make_link(
                "repo/docs/AGENTS.override.md",
                world.scratch.join("elsewhere.md"),
            )
            .expect("linked");
        let run = world.alz(&["build"]);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""));
        for path in ["build/AGENTS.override.md", "docs/AGENTS.override.md"] {
            let line = format!(
                "error [collision] {path}: AGENTS.override.md silently shadows the generated AGENTS.md; remove it\n"
            );
            assert!(run.stderr.contains(&line), "{path}: {}", run.stderr);
        }
        assert!(!world.scratch.exists("repo/AGENTS.md"));
        assert!(!world.scratch.exists("repo/layer.lock"));
    }

    /// Spec §4.3 "Writing": each file gets mode `0644`, or `0755` when it is executable, whatever
    /// the umask, and a missing folder is created with `0777` less the umask. The build runs under
    /// umasks that differ from the usual `022`, so code that leans on the umask fails here.
    #[test]
    fn files_keep_their_mode_and_new_folders_follow_the_umask_whatever_it_is() {
        for (umask, folder_mode) in [("077", 0o700), ("027", 0o750), ("000", 0o777)] {
            let world = World::new(&[
                ("instructions/00-core.md", "Keep it small.\n"),
                ("agents/helper.md", AGENT),
            ]);
            let run = world.build_under_umask(umask);
            assert_eq!((run.code, run.stderr.as_str()), (0, ""), "umask {umask}");
            for file in [
                "AGENTS.md",
                "CLAUDE.md",
                ".claude/agents/helper.md",
                "layer.lock",
            ] {
                assert_eq!(
                    world.scratch.mode(format!("repo/{file}")).expect("a mode") & 0o777,
                    0o644,
                    "{file} under umask {umask}"
                );
            }
            assert_eq!(
                world.scratch.mode("repo/.claude").expect("a mode") & 0o777,
                folder_mode,
                ".claude under umask {umask}"
            );
            assert_eq!(
                world.scratch.mode("repo/.claude/agents").expect("a mode") & 0o777,
                folder_mode,
                ".claude/agents under umask {umask}"
            );
        }
    }
}
