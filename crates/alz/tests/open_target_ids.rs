//! Open target ids (spec §5.2 **Target ids**, §8 **Targets without an adapter**, decision-40):
//! `targets` takes any well-formed id and an id with no adapter is the error `target`; a
//! `settings.yaml` section, a `models.yaml` alias key and a `targets/<id>/` folder of an id that
//! is not in `targets` are checked but never written and give no diagnostic.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A scratch copy of the settings fixture (a personal layer and a repository), with its own
    /// home and cache.
    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/37-settings-targets");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            Self {
                repository: scratch.join("work/repo"),
                scratch,
            }
        }

        fn write(&self, path: &str, text: &str) {
            if let Some(parent) = Path::new(path).parent() {
                self.scratch
                    .make_dirs(Path::new("work/repo").join(parent))
                    .expect("made");
            }
            self.scratch
                .write(format!("work/repo/{path}"), text)
                .expect("written");
        }

        fn targets(&self, list: &str) {
            self.write(
                "layer.yaml",
                &format!(
                    "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: {list}\n"
                ),
            );
        }

        fn build(&self) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("CODEX_HOME")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
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
    }

    /// Spec §8 **Targets without an adapter**: a well-formed id that no adapter renders is the
    /// error `target` at `layer.yaml` that lists the registered adapters, and nothing is written.
    #[test]
    fn a_target_id_without_an_adapter_is_the_error_target_listing_the_available_adapters() {
        let work = Workspace::new();
        work.targets("[antigravity]");
        let before = read_tree(&work.repository).expect("the tree is read");
        let run = work.build();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains(
                "error [target] layer.yaml: no adapter for 'antigravity' (available: claude)\n"
            ),
            "stderr: {}",
            run.stderr
        );
        assert_eq!(read_tree(&work.repository).expect("read"), before);
    }

    /// Spec §8 **Targets without an adapter**: each such target is reported once, with the
    /// available adapters in name order.
    #[test]
    fn each_target_without_an_adapter_is_reported_once() {
        let work = Workspace::new();
        work.targets("[claude, zed, zed, antigravity]");
        let run = work.build();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        for id in ["zed", "antigravity"] {
            let line = format!("no adapter for '{id}' (available: claude)");
            assert_eq!(run.stderr.matches(&line).count(), 1, "{}", run.stderr);
        }
    }

    /// Spec §8 **Targets without an adapter**: `codex` keeps its message and its exit code, and
    /// an id with no adapter has the same exit code.
    #[test]
    fn the_codex_target_keeps_its_message_and_exit_code() {
        let codex = Workspace::new();
        codex.targets("[codex]");
        let codex_run = codex.build();
        assert!(
            codex_run.stderr.contains(
                "error [target] layer.yaml: the 'codex' adapter is not implemented yet (implemented: claude)\n"
            ),
            "stderr: {}",
            codex_run.stderr
        );
        let other = Workspace::new();
        other.targets("[antigravity]");
        assert_eq!(codex_run.code, other.build().code);
        assert_eq!(codex_run.code, 1);
    }

    /// Spec §5.2 **Target ids**: an entry of `targets` that is not a target id is the manifest
    /// error that names `targets[<n>]`.
    #[test]
    fn a_target_that_is_not_a_target_id_is_a_manifest_error() {
        let work = Workspace::new();
        work.targets("[claude, My_Tool]");
        let run = work.build();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains("targets[1] must be a target id"),
            "stderr: {}",
            run.stderr
        );
    }

    /// Spec §5.2 **Target ids**, §7.8, §7.9, §7.11: a settings section, a model alias key and a
    /// `targets/` folder of an id that is not in `targets` build with no diagnostic and are never
    /// written.
    #[test]
    fn sections_for_an_id_that_is_not_built_for_give_no_diagnostic_and_are_never_written() {
        let plain = Workspace::new();
        plain.targets("[claude]");
        let baseline = plain.build();
        assert_eq!(baseline.code, 0, "{}{}", baseline.stdout, baseline.stderr);

        let work = Workspace::new();
        work.targets("[claude]");
        work.write(
            ".layer/settings.yaml",
            "antigravity:\n  theme: dark\n  hooks_dir: /x\n",
        );
        work.write(
            ".layer/models.yaml",
            "aliases:\n  strong: { claude: opus-model, antigravity: big-model }\n",
        );
        work.write(".layer/targets/antigravity/rules/note.json", "{\"a\": 1}\n");
        let run = work.build();
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert_eq!(run.stderr, "", "no diagnostic");
        // What the build made: every file outside the layer's own folder.
        let made = |repository: &Path| -> Vec<(String, String)> {
            read_tree(repository)
                .expect("the tree is read")
                .iter()
                .map(|(path, file)| {
                    (
                        format!("{path:?}"),
                        String::from_utf8_lossy(&file.bytes).into_owned(),
                    )
                })
                .filter(|(path, _)| !path.contains(".layer"))
                .collect()
        };
        let written = made(&work.repository);
        let expected = made(&plain.repository);
        let paths = |files: &[(String, String)]| -> Vec<String> {
            files.iter().map(|(path, _)| path.clone()).collect()
        };
        assert_eq!(
            paths(&written),
            paths(&expected),
            "the same files as a build with no section of antigravity"
        );
        for (path, text) in &written {
            assert!(
                !text.contains("big-model")
                    && !text.contains("hooks_dir")
                    && !path.contains("note.json"),
                "{path} holds a section of antigravity"
            );
        }
        assert_eq!(baseline.stdout.lines().count(), run.stdout.lines().count());
    }

    /// Spec §7.9: a top-level key of `settings.yaml` that is not a target id is still the error
    /// `settings`, now that every well-formed id is a section.
    #[test]
    fn a_settings_key_that_is_not_a_target_id_is_still_an_error() {
        let work = Workspace::new();
        work.targets("[claude]");
        work.write(".layer/settings.yaml", "Antigravity:\n  theme: dark\n");
        let run = work.build();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(run.stderr.contains("error [settings]"), "{}", run.stderr);
        assert!(run.stderr.contains("'Antigravity'"), "{}", run.stderr);
    }

    /// Spec §7.11: a file below a folder of `targets/` that is not a target id is the error
    /// `passthrough` with the message of §5.2.
    #[test]
    fn a_folder_of_targets_that_is_not_a_target_id_is_a_passthrough_error() {
        let work = Workspace::new();
        work.targets("[claude]");
        work.write(".layer/targets/My_Tool/x.json", "{}\n");
        let run = work.build();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains(
                "this file is below a folder of targets/ that is not a target id (\u{a7}5.2)"
            ),
            "{}",
            run.stderr
        );
    }
}
