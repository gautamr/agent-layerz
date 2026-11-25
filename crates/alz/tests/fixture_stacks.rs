//! Fixtures of the layout, manifest and resolution sections run through the real `alz` binary
//! (TASK-55.9.6): `11-visibility` (spec §6 "Validating the stack", §3.1), `33-name-order` (§6
//! "Name order") and `35-manifest-schema` (§5.1, §5.2). Each fixture is copied into a scratch
//! directory with an isolated `HOME` and cache, and is never edited in place.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_io::EnvSnapshot;
    use alz_test_support::{ScratchDir, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct Workspace {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl Workspace {
        fn of(fixture: &str) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures")
                    .join(fixture),
                &root.join("work"),
            )
            .expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            Self {
                repository: root.join("work/repo"),
                scratch,
            }
        }

        fn alz(&self, args: &[&str]) -> Run {
            let root = self.scratch.real_path().expect("a real path");
            let path = EnvSnapshot::capture()
                .vars()
                .get("PATH")
                .expect("PATH is set")
                .to_owned();
            let mut command: Command = cargo_bin_cmd!("alz");
            let output = command
                .current_dir(&self.repository)
                .env_clear()
                .env("PATH", path)
                .env("HOME", root.join("home"))
                .env("XDG_CACHE_HOME", root.join("xdg"))
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

        fn read(&self, relative: &str) -> String {
            let root = self.scratch.real_path().expect("a real path");
            self.scratch
                .read_text(root.join("work/repo").join(relative))
                .expect("a file")
        }

        fn put(&self, relative: &str, text: &str) {
            let root = self.scratch.real_path().expect("a real path");
            let path = root.join("work").join(relative);
            if let Some(parent) = path.parent() {
                self.scratch.make_dirs(parent).expect("made");
            }
            self.scratch.write(path, text).expect("written");
        }
    }

    /// Spec §6 "Validating the stack" and §3.1 (fixture `11-visibility`): a layer that is more
    /// public than a layer before it is the error `visibility` at the later layer's entry. The
    /// fixture lists a private client layer before a public personal layer.
    #[test]
    fn a_public_layer_after_a_private_one_is_refused_with_the_order_to_use() {
        let workspace = Workspace::of("11-visibility");

        let run = workspace.alz(&["lint"]);

        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stdout.starts_with("lint failed: 1 error"),
            "{}",
            run.stdout
        );
        assert_eq!(
            run.stderr,
            "error [visibility] path:../personal: public layer 'personal' cannot extend private layer 'acme'; order the stack from most public to most private\n"
        );
    }

    /// Spec §6 "Validating the stack" (fixture `11-visibility`): the same layers ordered from
    /// most public to most private are a valid stack.
    #[test]
    fn the_stack_ordered_from_most_public_to_most_private_is_valid() {
        let workspace = Workspace::of("11-visibility");
        workspace.put(
            "repo/layer.yaml",
            "stack:\n  - source: path:../personal\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n",
        );

        let run = workspace.alz(&["lint"]);

        assert_eq!((run.code, run.stderr.as_str()), (0, ""), "{}", run.stdout);
    }

    /// Spec §6 "Name order" (fixture `33-name-order`): the instruction files of a layer are
    /// merged in the code point order of their names: `-` before `.`, upper case before lower
    /// case, `10` before `9`, and a name above U+007F last. The build is the same twice.
    #[test]
    fn instruction_files_are_merged_in_code_point_order_and_the_build_is_deterministic() {
        let workspace = Workspace::of("33-name-order");
        for (name, text) in [
            ("a.md", "BLOCK-DOT"),
            ("a-b.md", "BLOCK-DASH"),
            ("Z.md", "BLOCK-UPPER"),
            ("10.md", "BLOCK-TEN"),
            ("9.md", "BLOCK-NINE"),
            ("\u{e9}.md", "BLOCK-ACCENT"),
        ] {
            workspace.put(&format!("repo/.layer/instructions/{name}"), text);
        }

        let first = workspace.alz(&["build"]);
        assert_eq!(
            (first.code, first.stderr.as_str()),
            (0, ""),
            "{}",
            first.stdout
        );
        let text = workspace.read("AGENTS.md");
        let positions = [
            "BLOCK-TEN",
            "BLOCK-NINE",
            "BLOCK-UPPER",
            "BLOCK-DASH",
            "BLOCK-DOT",
            "BLOCK-ACCENT",
        ]
        .map(|marker| {
            text.find(marker)
                .unwrap_or_else(|| panic!("{marker} in {text}"))
        });
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "{positions:?}\n{text}"
        );
        assert!(text.find("Base instructions.").expect("the personal block") < positions[0]);

        let lock = workspace.read("layer.lock");
        assert_eq!(workspace.alz(&["build"]).code, 0);
        assert_eq!(workspace.read("AGENTS.md"), text);
        assert_eq!(workspace.read("layer.lock"), lock);
    }

    /// Spec §5.2 (fixture `35-manifest-schema`): a manifest with only `stack` and `targets`
    /// is valid; `lint` prints the layers in stack order.
    #[test]
    fn the_manifest_of_the_fixture_is_valid_and_names_the_layers_in_stack_order() {
        let workspace = Workspace::of("35-manifest-schema");

        let run = workspace.alz(&["lint"]);

        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal, repo\n");
    }

    /// Spec §5.2 "Field rules" (fixture `35-manifest-schema`): a field the schema does not have
    /// is refused as a quoted key, and the value is never printed.
    #[test]
    fn an_unknown_manifest_field_is_refused_by_its_key_and_never_its_value() {
        let workspace = Workspace::of("35-manifest-schema");
        workspace.put(
            "repo/layer.yaml",
            "stack:\n  - source: path:../personal\n  - source: path:./.layer\nbogus: SECRET-VALUE\n",
        );

        let run = workspace.alz(&["lint"]);

        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [manifest] layer.yaml"),
            "{}",
            run.stderr
        );
        assert!(run.stderr.contains("'bogus'"), "{}", run.stderr);
        assert!(!run.stderr.contains("SECRET-VALUE"), "{}", run.stderr);
        assert_eq!(run.stdout, "");
    }

    /// Spec §5.1 "Field rules" (fixture `35-manifest-schema`): a layer whose `kind` is missing
    /// is the error `layer`, and the layer is named by its stack entry.
    #[test]
    fn a_layer_without_its_required_fields_is_refused_naming_each_field() {
        let workspace = Workspace::of("35-manifest-schema");
        workspace.put("repo/.layer/layer.yaml", "name: repo\n");

        let run = workspace.alz(&["lint"]);

        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains("kind") && run.stderr.contains("visibility"),
            "{}",
            run.stderr
        );
        assert!(run.stderr.contains("path:./.layer"), "{}", run.stderr);
    }

    /// Spec §4.3 "Which files are alz's own" (fixture `14-non-destructive`): a hand-written file
    /// in a folder the build manages, which the build does not make, is a collision. The build
    /// stops with exit code 1 and writes nothing: the whole tree is the same before and after.
    #[test]
    fn a_hand_written_file_in_a_managed_folder_stops_the_build_and_nothing_is_written() {
        let workspace = Workspace::of("14-non-destructive");
        let before = read_tree(&workspace.repository).expect("the tree");

        let run = workspace.alz(&["build"]);

        assert_eq!(run.code, 1, "{}", run.stdout);
        assert!(
            run.stderr.contains(".claude/agents/foo.md"),
            "{}",
            run.stderr
        );
        assert!(!run.stderr.contains("HAND-WRITTEN"), "{}", run.stderr);
        assert_eq!(read_tree(&workspace.repository).expect("the tree"), before);
    }
}
