//! The YAML limits at the command line (spec §7.13 "YAML limits" and "What else makes a YAML file
//! invalid"): every YAML file `alz` reads, `layer.yaml`, the frontmatter of a layer's files and its
//! settings files, is capped at 1 MiB and parsed with the alias limit, and the rules of the tags,
//! duplicate keys and documents hold for each. Run through `alz lint` against a copy of
//! `fixtures/34-yaml-limits`; the budgets themselves are proved by `alz-core`'s yaml tests.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A copy of fixture 34, a layer, in an isolated world.
    struct World {
        scratch: ScratchDir,
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/34-yaml-limits/layer");
            copy_tree(&fixture, &scratch.join("layer")).expect("the fixture is copied");
            for folder in ["home", "config", "state", "cache"] {
                scratch.make_dir(folder).expect("made");
            }
            Self { scratch }
        }

        fn lint(&self) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(self.scratch.join("layer"))
                .env_clear()
                .env("PATH", self.scratch.join("none"))
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
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

    /// `count` lines that use the alias `*a`, as the entries of a mapping.
    fn alias_uses(count: usize, indent: &str) -> String {
        (0..count).fold(String::new(), |text, index| {
            text + indent + "k" + &index.to_string() + ": *a\n"
        })
    }

    /// The files that hold YAML in a layer, each with the text that puts `uses` aliases in it where
    /// the schema allows a list or a mapping of text: the manifest's `requires_slots` and the
    /// `claude.env` of `settings.yaml`.
    fn files_with_aliases(uses: usize) -> [(&'static str, String); 2] {
        [
            (
                "layer.yaml",
                format!(
                    "name: limits\nkind: personal\nvisibility: public\nrequires_slots:\n  - &a slot\n{}",
                    (0..uses).map(|_| "  - *a\n").collect::<String>()
                ),
            ),
            (
                "settings.yaml",
                format!("claude:\n  env:\n    A: &a x\n{}", alias_uses(uses, "    ")),
            ),
        ]
    }

    /// Spec §7.13 "YAML limits": at most 100 aliases in a document, in the manifest, in a
    /// frontmatter block and in a settings file alike.
    #[test]
    fn a_document_with_100_aliases_lints_and_one_with_101_is_refused_in_every_kind_of_file() {
        let frontmatter = World::new();
        frontmatter
            .scratch
            .write(
                "layer/instructions/10-more.md",
                format!("---\nmode: &a replace\n{}---\nbody\n", alias_uses(101, "")),
            )
            .expect("written");
        let run = frontmatter.lint();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr
                .contains("error [yaml] limits:instructions/10-more.md:")
                && run.stderr.contains("more than 100 aliases"),
            "{}",
            run.stderr
        );

        for (file, text) in files_with_aliases(100) {
            let world = World::new();
            world
                .scratch
                .write(format!("layer/{file}"), text)
                .expect("written");
            let run = world.lint();
            assert_eq!(
                run.code, 0,
                "{file} with 100 aliases: {}{}",
                run.stdout, run.stderr
            );
        }
        for (file, text) in files_with_aliases(101) {
            let world = World::new();
            world
                .scratch
                .write(format!("layer/{file}"), text)
                .expect("written");
            let run = world.lint();
            assert_eq!(
                run.code, 1,
                "{file} with 101 aliases: {}{}",
                run.stdout, run.stderr
            );
            assert!(
                run.stderr.contains("error [yaml]") && run.stderr.contains("more than 100 aliases"),
                "{file}: {}",
                run.stderr
            );
            assert!(run.stderr.contains(file), "{file} is named: {}", run.stderr);
        }
    }

    /// Spec §7.13 "YAML limits": a file past 1 MiB counts as invalid YAML, and the 1 MiB counts
    /// the whole file that holds a frontmatter block.
    #[test]
    fn a_frontmatter_file_over_one_mebibyte_is_invalid_yaml_and_one_of_exactly_that_size_is_not() {
        let head = "---\nmode: replace\n# ";
        let tail = "\n---\nbody\n";
        let exactly = 1024 * 1024;
        for (size, code) in [(exactly, 0), (exactly + 1, 1)] {
            let world = World::new();
            let filler = "a".repeat(size - head.len() - tail.len());
            world
                .scratch
                .write(
                    "layer/instructions/10-big.md",
                    format!("{head}{filler}{tail}"),
                )
                .expect("written");
            let run = world.lint();
            assert_eq!(run.code, code, "{size} bytes: {}{}", run.stdout, run.stderr);
            if code == 1 {
                assert!(
                    run.stderr.contains(
                        "error [yaml] limits:instructions/10-big.md: the file is larger than 1 MiB"
                    ),
                    "{}",
                    run.stderr
                );
            }
        }
    }

    /// Spec §7.13 "What else makes a YAML file invalid": a tag outside the core set (`yaml-tag`), a
    /// duplicate key, a second document and an alias without an anchor are errors in a settings
    /// file, and the error never quotes the tag, the alias name or the value; a duplicate key is
    /// the one thing it quotes.
    #[test]
    fn a_tag_a_second_document_an_unresolved_alias_and_a_duplicate_key_are_errors_that_quote_nothing()
     {
        let cases = [
            (
                "a: !!binary secret-value\n",
                "error [yaml-tag]",
                "a tag outside the YAML 1.2 core set",
                "binary",
            ),
            ("a: 1\n---\nb: 2\n", "error [yaml]", "", "b: 2"),
            ("a: *nowhere\n", "error [yaml]", "", "nowhere"),
            ("a: 1\na: 2\n", "error [yaml]", "duplicate key 'a'", "a: 2"),
        ];
        for (text, code, message, never) in cases {
            let world = World::new();
            world
                .scratch
                .write("layer/settings.yaml", text)
                .expect("written");
            let run = world.lint();
            assert_eq!(run.code, 1, "{text:?}: {}{}", run.stdout, run.stderr);
            assert!(
                run.stderr
                    .contains(&format!("{code} limits:settings.yaml:")),
                "{text:?}: {}",
                run.stderr
            );
            assert!(run.stderr.contains(message), "{text:?}: {}", run.stderr);
            assert!(
                !run.stderr.contains(never),
                "{text:?} is quoted: {}",
                run.stderr
            );
            assert!(!run.stderr.contains("secret-value"));
        }
    }

    /// Spec §7.13 "What else makes a YAML file invalid", "Control characters": a control character
    /// that an escape produces is an error in a file's keys and values, while tab, line feed and a
    /// space written as an escape are not.
    #[test]
    fn a_control_character_made_by_an_escape_is_an_error_and_tab_and_line_feed_are_not() {
        let refused = World::new();
        refused
            .scratch
            .write(
                "layer/settings.yaml",
                "claude:\n  env:\n    A: \"x\\x07y\"\n",
            )
            .expect("written");
        let run = refused.lint();
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains("error [yaml] limits:settings.yaml:"),
            "{}",
            run.stderr
        );

        let accepted = World::new();
        accepted
            .scratch
            .write(
                "layer/settings.yaml",
                "claude:\n  env:\n    A: \"x\\ty\\nz\"\n",
            )
            .expect("written");
        let run = accepted.lint();
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    }
}
