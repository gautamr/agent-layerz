//! The messages of stack resolution that spec §6 "Other messages of resolution", "The layer
//! cache" and "Layers in or around an account folder" give for a layer, a lock or a working
//! directory that cannot be used (TASK-55.9.6). Each test plants its own machine in a scratch
//! directory, runs the real `alz lint` with a cleared environment and an isolated `HOME`, and
//! asserts the exit code, the streams and what is on disk.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use alz_io::{EnvSnapshot, StackResolver, WorkingDirectory};
    use alz_test_support::{ScratchDir, running_as_root};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const LAYER: &str = "name: team\nkind: client\nvisibility: private\n";

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

    struct Machine {
        scratch: ScratchDir,
        root: PathBuf,
    }

    impl Machine {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            for name in ["home", "repo", "xdg"] {
                scratch.make_dir(name).expect("made");
            }
            Self { scratch, root }
        }

        fn manifest(&self, text: &str) {
            self.scratch
                .write("repo/layer.yaml", text)
                .expect("a manifest");
        }

        fn put(&self, relative: &str, text: &str) {
            if let Some(parent) = std::path::Path::new(relative).parent() {
                self.scratch.make_dirs(parent).expect("made");
            }
            self.scratch.write(relative, text).expect("written");
        }

        /// `alz lint` in `repo` with only `PATH`, `HOME` and `XDG_CACHE_HOME` set.
        fn lint(&self) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            let output = command
                .current_dir(self.root.join("repo"))
                .env_clear()
                .env("PATH", real_path())
                .env("HOME", self.root.join("home"))
                .env("XDG_CACHE_HOME", self.root.join("xdg"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", self.root.join("gitconfig"))
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
    }

    /// Spec §6 "Validating the stack": an unpinned `path:` layer that holds a name that is not a
    /// safe path (here a backslash) is the error `layer`, with no location, and the message names
    /// the layer by its source and never the name.
    #[test]
    fn an_unpinned_path_layer_with_a_name_that_is_not_allowed_is_the_error_layer() {
        let machine = Machine::new();
        machine.put("bad/layer.yaml", LAYER);
        machine.put("bad/a\\b.md", "text\n");
        machine.manifest("stack:\n  - source: path:../bad\n");

        let run = machine.lint();

        assert_eq!(run.code, 1);
        assert!(
            run.stdout.starts_with("lint failed: 1 error in "),
            "{}",
            run.stdout
        );
        assert_eq!(
            run.stderr,
            "error [layer] layer 'path:../bad' holds a file that cannot be read or a name that is not allowed\n"
        );
    }

    /// Spec §6 "Validating the stack": an unpinned `path:` layer that holds a file that cannot be
    /// read is the same error `layer` as a name that is not allowed.
    #[test]
    fn an_unpinned_path_layer_with_a_file_that_cannot_be_read_is_the_error_layer() {
        let machine = Machine::new();
        machine.put("bad/layer.yaml", LAYER);
        machine.put("bad/rules.md", "text\n");
        machine
            .scratch
            .set_mode("bad/rules.md", 0o000)
            .expect("made unreadable");
        if running_as_root() {
            return;
        }
        machine.manifest("stack:\n  - source: path:../bad\n");

        let run = machine.lint();

        assert_eq!(run.code, 1);
        assert!(
            run.stdout.starts_with("lint failed: 1 error in "),
            "{}",
            run.stdout
        );
        assert_eq!(
            run.stderr,
            "error [layer] layer 'path:../bad' holds a file that cannot be read or a name that is not allowed\n"
        );
    }

    /// Spec §6 "Other messages of resolution": a `layer.lock` that cannot be read is the error
    /// `lock` at `layer.lock`.
    #[test]
    fn a_layer_lock_that_cannot_be_read_is_the_error_lock_cannot_be_read() {
        let machine = Machine::new();
        machine.put("good/layer.yaml", LAYER);
        machine.manifest("stack:\n  - source: path:../good\n");
        machine.put("repo/layer.lock", "{}\n");
        machine
            .scratch
            .set_mode("repo/layer.lock", 0o000)
            .expect("made unreadable");
        if running_as_root() {
            return;
        }

        let run = machine.lint();

        assert_eq!(run.code, 1);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            "error [lock] layer.lock: layer.lock cannot be read\n"
        );
    }

    /// Spec §6 "Other messages of resolution": a process that has no working directory cannot
    /// resolve the stack: the error `source`, no location, `cannot find the working directory`.
    /// A command that has no working directory cannot read `layer.yaml` either, so the
    /// resolver is asked directly, with the working directory a run holds when it has none.
    #[test]
    fn a_resolver_without_a_working_directory_is_the_error_source_cannot_find_it() {
        let diagnostic = StackResolver::in_working_directory(
            &EnvSnapshot::capture(),
            &WorkingDirectory::default(),
        )
        .expect_err("no working directory, no resolver");

        assert_eq!(
            diagnostic.to_string(),
            "error [source] cannot find the working directory"
        );
    }

    /// Spec §6 "Name order": a name or key that holds a lone surrogate, for example from a YAML
    /// `"\uD800"` escape, has no UTF-8 form and is an error: the run ends with exit code 1 and
    /// one error line, never with a panic, and the value is not echoed.
    #[test]
    fn a_key_with_a_lone_surrogate_from_a_yaml_escape_is_an_error() {
        let machine = Machine::new();
        machine.put(
            "bad/layer.yaml",
            "name: team\nkind: client\nvisibility: private\n\"\\uD800\": x\n",
        );
        machine.manifest("stack:\n  - source: path:../bad\n");

        let run = machine.lint();

        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(run.stderr.starts_with("error ["), "{}", run.stderr);
        assert!(!run.stderr.contains("panicked"), "{}", run.stderr);
        assert!(!run.stderr.contains("uD800"), "{}", run.stderr);
    }
}
