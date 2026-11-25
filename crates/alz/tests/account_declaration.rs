//! The `account` field of a layer at the command line (spec §7.13: "Account names", "Who may
//! declare it", "No generated output", "Environment" and the off-by-default rule), against
//! `fixtures/29-account-layer`. The fixture is copied into a scratch folder for each run; each run
//! has its own `HOME` and `XDG_*` folders and a `PATH` with no tool in it that the build needs.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
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

    /// A copy of fixture 29 in an isolated world.
    struct World {
        scratch: ScratchDir,
    }

    const THREE: &str = "stack:\n  - source: path:../personal\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n";

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/29-account-layer");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            for folder in ["home", "config", "state", "cache"] {
                scratch.make_dir(folder).expect("made");
            }
            Self { scratch }
        }

        fn repo(&self) -> PathBuf {
            self.scratch.join("work/repo")
        }

        /// Adds a line to a layer's `layer.yaml`, a path below `work/`.
        fn append(&self, layer_yaml: &str, text: &str) {
            let path = format!("work/{layer_yaml}");
            let mut now = self.scratch.read_text(&path).expect("the layer file");
            now.push_str(text);
            self.scratch.write(path, now).expect("written");
        }

        fn stack(&self, manifest: &str) {
            self.scratch
                .write("work/repo/layer.yaml", manifest)
                .expect("the manifest is written");
        }

        fn alz_in(&self, directory: &Path, arguments: &[&str], extra: &[(&str, &str)]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(directory)
                .env_clear()
                .env("PATH", self.scratch.join("none"))
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .timeout(Duration::from_secs(60))
                .args(arguments);
            for (name, value) in extra {
                alz.env(name, value);
            }
            let output = alz.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn alz(&self, arguments: &[&str]) -> Run {
            self.alz_in(&self.repo(), arguments, &[])
        }
    }

    /// Spec §7.13 "Who may declare it": a declaration by any layer but the first one and a private
    /// `kind: client` layer outside the repository is a build error that names the layer and the
    /// reason, and a failed build writes nothing.
    #[test]
    fn a_layer_that_may_not_declare_an_account_fails_the_build_naming_it_and_writes_nothing() {
        let cases: [(&str, &str, &str, &str); 4] = [
            (
                "group/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:../group\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n",
                "error [account] group:layer.yaml: layer 'group' cannot declare account: its kind is 'group', not 'client'.",
                "a group layer",
            ),
            (
                "public-client/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:../public-client\n  - source: path:./.layer\ntargets: [claude]\n",
                "error [account] public-client:layer.yaml: layer 'public-client' cannot declare account: it is public.",
                "a public layer that calls itself client",
            ),
            (
                "repo/.layer/layer.yaml",
                THREE,
                "error [account] repo:layer.yaml: layer 'repo' cannot declare account: it is inside the repository;",
                "the repository's own layer",
            ),
            (
                "client-b/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:../client\n  - source: path:../client-b\n  - source: path:./.layer\ntargets: [claude]\n",
                "error [account] client-b:layer.yaml: layers 'client' and 'client-b' both declare account; only one client layer may",
                "a second client layer",
            ),
        ];
        for (layer, manifest, message, what) in cases {
            let world = World::new();
            world.stack(manifest);
            world.append(layer, "account: acme\n");
            if layer == "client-b/layer.yaml" {
                world.append("client/layer.yaml", "account: acme\n");
            }
            let before = read_tree(&world.scratch.join("work")).expect("read");
            let run = world.alz(&["build"]);
            assert_eq!(run.code, 1, "{what}: {}{}", run.stdout, run.stderr);
            assert!(run.stderr.contains(message), "{what}: {}", run.stderr);
            assert_eq!(
                read_tree(&world.scratch.join("work")).expect("read"),
                before,
                "{what}: a failed build writes nothing"
            );
        }
    }

    /// Spec §7.13 "Who may declare it": the first stack entry and a private client layer outside
    /// the repository may both declare, and the client layer's value replaces the first layer's,
    /// so that the build has no error.
    #[test]
    fn the_first_layer_and_a_private_client_layer_may_both_declare() {
        let world = World::new();
        world.append("personal/layer.yaml", "account: personal-acme\n");
        world.append("client/layer.yaml", "account:\n  claude: client-acme\n");
        let run = world.alz(&["build"]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert_eq!(run.stderr, "");
    }

    /// Spec §7.13 "No generated output": the build writes no account value into any generated file
    /// or the lock, so the output is byte-identical whatever account a layer names.
    #[test]
    fn a_declared_account_changes_no_generated_byte_and_no_line_of_the_report() {
        let without = World::new();
        let with = World::new();
        with.append("client/layer.yaml", "account: acme-secret-name\n");
        let plain = without.alz(&["build"]);
        let named = with.alz(&["build"]);
        assert_eq!(plain.code, 0, "{}", plain.stderr);
        assert_eq!(named.code, 0, "{}", named.stderr);
        assert_eq!(named.stdout, plain.stdout);
        assert_eq!(named.stderr, plain.stderr);
        // The lock records a content hash of each layer, which covers its `layer.yaml`, so the hash
        // of the client layer differs; the value itself is in no generated file and not in the lock.
        let generated = |world: &World| {
            read_tree(&world.repo())
                .expect("read")
                .into_iter()
                .filter(|(path, _)| !path.starts_with(".layer/") && path != "layer.lock")
                .map(|(path, file)| (path, file.bytes))
                .collect::<Vec<_>>()
        };
        let generated_with = generated(&with);
        assert!(
            generated_with.iter().any(|(path, _)| path == "AGENTS.md"),
            "the build writes AGENTS.md"
        );
        assert!(
            generated_with == generated(&without),
            "the generated files differ when a layer names an account"
        );
        let tree = read_tree(&with.repo()).expect("read");
        assert!(tree.contains_key("layer.lock"), "the build writes a lock");
        assert!(
            tree.iter()
                .filter(|(path, _)| !path.starts_with(".layer/"))
                .all(|(_, file)| !String::from_utf8_lossy(&file.bytes).contains("acme-secret-name")),
            "no generated file or the lock holds the account name"
        );
    }

    /// Spec §7.13 introduction: with no layer declaration and no user config, alz prints nothing
    /// about accounts and `alz account which` reports the default account with the variable unset.
    #[test]
    fn with_no_declaration_alz_prints_nothing_about_accounts() {
        let world = World::new();
        for arguments in [["build"], ["check"]] {
            let run = world.alz(&arguments);
            assert_eq!(run.code, 0, "{arguments:?}: {}{}", run.stdout, run.stderr);
            let text = format!("{}{}", run.stdout, run.stderr).to_lowercase();
            assert!(
                !text.contains("account"),
                "{arguments:?} says something about accounts: {text}"
            );
        }
        let which = world.alz(&["account", "which"]);
        assert_eq!(which.code, 0, "{}", which.stderr);
        assert!(
            which
                .stdout
                .contains("claude: default (source: default) CLAUDE_CONFIG_DIR unset"),
            "{}",
            which.stdout
        );
    }

    /// Spec §7.13 "Account names": a value that is not an account name, or a map key that is not a
    /// target, is a schema error of `layer.yaml` in `alz build` and in `alz lint`, and the value
    /// is never printed.
    #[test]
    fn an_account_value_that_breaks_the_name_rule_is_a_schema_error_and_is_never_printed() {
        let sixty_five = "a".repeat(65);
        let values = [
            "Bad_Name".to_owned(),
            "../escape".to_owned(),
            "-leading".to_owned(),
            "has space".to_owned(),
            sixty_five,
            "{cursor: tool-user}".to_owned(),
            "{claude: Bad_Name}".to_owned(),
        ];
        for value in values {
            let world = World::new();
            world.append("client/layer.yaml", &format!("account: {value}\n"));
            let build = world.alz(&["build"]);
            assert_eq!(build.code, 1, "{value}: {}{}", build.stdout, build.stderr);
            assert!(
                build
                    .stderr
                    .contains("error [layer] path:../client:layer.yaml:"),
                "{value}: {}",
                build.stderr
            );
            let lint = world.alz_in(&world.scratch.join("work/client"), &["lint"], &[]);
            assert_eq!(lint.code, 1, "{value}: {}{}", lint.stdout, lint.stderr);
            for text in [&build.stderr, &build.stdout, &lint.stderr, &lint.stdout] {
                for fragment in ["Bad_Name", "escape", "-leading", "has space", "tool-user"] {
                    if value.contains(fragment) {
                        assert!(!text.contains(fragment), "{value} is repeated: {text}");
                    }
                }
                assert!(!text.contains(&"a".repeat(65)), "a long name is repeated");
            }
        }
    }

    /// Spec §7.13 "Environment": resolution reads exactly the named variables. An `ALZ_USE_<TARGET>`
    /// variable, an agent-session marker and the `GIT_*` variables change neither the account nor
    /// what `alz account which` prints.
    #[test]
    fn no_variable_but_the_ones_the_spec_names_changes_the_resolved_account() {
        let world = World::new();
        let plain = world.alz_in(&world.repo(), &["account", "which"], &[]);
        assert_eq!(plain.code, 0, "{}", plain.stderr);
        let noisy = world.alz_in(
            &world.repo(),
            &["account", "which"],
            &[
                ("ALZ_USE_CLAUDE", "work"),
                ("ALZ_USE_CODEX", "work"),
                ("CLAUDECODE", "1"),
                ("CODEX_THREAD_ID", "t"),
                ("AI_AGENT", ""),
                ("GIT_DIR", "/nonexistent"),
                ("GIT_CONFIG_GLOBAL", "/nonexistent"),
                ("GIT_AUTHOR_NAME", "Somebody"),
            ],
        );
        assert_eq!(noisy.code, 0, "{}", noisy.stderr);
        assert_eq!(noisy.stdout, plain.stdout);
        assert_eq!(noisy.stderr, plain.stderr);
    }
}
