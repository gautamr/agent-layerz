//! `alz build` with a `settings.yaml` (spec §7.9, fixture `37-settings-targets`): the top level
//! takes only target names, a refused file merges none of its settings and writes nothing, and the
//! settings of two layers deep-merge into `.claude/settings.json`.

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

    /// A scratch copy of the fixture, with its own home and cache.
    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn with(personal: Option<&str>, repository: Option<&str>) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/37-settings-targets");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            if let Some(text) = personal {
                scratch
                    .write("work/personal/settings.yaml", text)
                    .expect("written");
            }
            if let Some(text) = repository {
                scratch
                    .write("work/repo/.layer/settings.yaml", text)
                    .expect("written");
            }
            Self {
                repository: scratch.join("work/repo"),
                scratch,
            }
        }

        fn build(&self) -> Run {
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
                .arg("build")
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn settings_json(&self) -> String {
            String::from_utf8(
                self.scratch
                    .read("work/repo/.claude/settings.json")
                    .expect("settings.json is written"),
            )
            .expect("UTF-8")
        }
    }

    /// Spec §7.9: any top-level key that is not a target name is the error `settings`, naming the
    /// layer and the file with the key quoted, and none of that file's settings are merged, so a
    /// valid `claude:` section beside it is not written either.
    #[test]
    fn a_top_level_key_that_is_no_target_is_refused_and_nothing_of_the_file_is_written() {
        let workspace = Workspace::with(
            None,
            Some("claude:\n  cleanupPeriodDays: 14\nBogus:\n  key: value\n"),
        );
        let before = read_tree(&workspace.repository).expect("read");
        let run = workspace.build();
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stderr.contains("error [settings]")
                && run.stderr.contains("repo:settings.yaml")
                && run.stderr.contains("'Bogus'"),
            "{}",
            run.stderr
        );
        assert!(
            !run.stderr.contains("value"),
            "the value is never printed: {}",
            run.stderr
        );
        assert_eq!(read_tree(&workspace.repository).expect("read"), before);
        assert!(run.stdout.is_empty(), "{}", run.stdout);
    }

    /// Spec §7.9: the value of a target is a mapping; `null` or any other value is the error
    /// `settings` and nothing is written.
    #[test]
    fn a_target_whose_value_is_not_a_mapping_is_refused_and_nothing_is_written() {
        for settings in ["claude: null\n", "claude: [a, b]\n", "claude: 3\n"] {
            let workspace = Workspace::with(Some(settings), None);
            let before = read_tree(&workspace.repository).expect("read");
            let run = workspace.build();
            assert_eq!(run.code, 1, "{settings}: {}", run.stderr);
            assert!(
                run.stderr.contains("error [settings]")
                    && run.stderr.contains("personal:settings.yaml"),
                "{settings}: {}",
                run.stderr
            );
            assert_eq!(
                read_tree(&workspace.repository).expect("read"),
                before,
                "{settings}"
            );
        }
    }

    /// Spec §7.9: mappings merge key by key, arrays concatenate without duplicates, and the more
    /// specific scalar wins; the merged `claude:` settings are written to `.claude/settings.json`.
    #[test]
    fn the_settings_of_two_layers_merge_deeply_into_claude_settings_json() {
        let workspace = Workspace::with(
            Some("claude:\n  cleanupPeriodDays: 14\n  env:\n    A: \"1\"\n    SHARED: personal\n"),
            Some("claude:\n  cleanupPeriodDays: 30\n  env:\n    B: \"2\"\n    SHARED: repo\n"),
        );
        let run = workspace.build();
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            workspace.settings_json(),
            "{\n  \"cleanupPeriodDays\": 30,\n  \"env\": {\n    \"A\": \"1\",\n    \"B\": \"2\",\n    \"SHARED\": \"repo\"\n  }\n}\n"
        );
    }
}
