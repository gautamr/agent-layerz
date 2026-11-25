//! Spec §13 items 12 and 24 through `alz build` (TASK-55.9.9). Item 12, "Account independence":
//! the build, whose effective agents and skills are its output, does not depend on the account
//! folder `CLAUDE_CONFIG_DIR` names. Item 24, "Untrusted repository", account cases: a layer that
//! may not declare `account` and does, fails the build, names the layers and writes nothing, while
//! the one allowed client layer builds. Fixture `29-account-layer`; every run has an isolated
//! home, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, TreeFile, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stderr: String,
    }

    /// A scratch copy of `fixtures/29-account-layer` with the manifest of the repository
    /// replaced, and `account: client-a` added to the layers named.
    struct World {
        scratch: ScratchDir,
        repository: PathBuf,
    }

    impl World {
        fn new(stack: &[&str], declaring: &[&str]) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/29-account-layer");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            let entries: String = stack
                .iter()
                .map(|source| ["  - source: path:", source, "\n"].concat())
                .collect();
            scratch
                .write(
                    "work/repo/layer.yaml",
                    format!("stack:\n{entries}targets: [claude]\n"),
                )
                .expect("written");
            for layer in declaring {
                let file = if *layer == "repo" {
                    "work/repo/.layer/layer.yaml".to_owned()
                } else {
                    format!("work/{layer}/layer.yaml")
                };
                let text = scratch.read_text(&file).expect("read");
                scratch
                    .write(&file, format!("{text}account: client-a\n"))
                    .expect("written");
            }
            for folder in ["home", "cache", "state", "config"] {
                scratch.make_dirs(folder).expect("made");
            }
            Self {
                repository: scratch.join("work/repo"),
                scratch,
            }
        }

        fn alz(&self, arguments: &[&str], account_folder: Option<&str>) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            command
                .current_dir(&self.repository)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CONFIG_HOME", self.scratch.join("config"))
                .env("XDG_STATE_HOME", self.scratch.join("state"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(arguments);
            if let Some(folder) = account_folder {
                command.env("CLAUDE_CONFIG_DIR", self.scratch.join(folder));
            }
            let output = command.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn tree(&self) -> BTreeMap<String, TreeFile> {
            read_tree(&self.repository).expect("the repository is read")
        }
    }

    const CLIENT_STACK: [&str; 3] = ["../personal", "../client", "./.layer"];

    /// Spec §13 item 12: with `CLAUDE_CONFIG_DIR` at two different empty account folders, the
    /// build writes the same agents, skills and every other file, and `alz check` passes under
    /// the other folder.
    #[test]
    fn the_build_is_the_same_under_two_different_empty_account_folders() {
        let first = World::new(&CLIENT_STACK, &[]);
        first.scratch.make_dirs("account-one").expect("made");
        let second = World::new(&CLIENT_STACK, &[]);
        second.scratch.make_dirs("account-two").expect("made");
        assert_eq!(first.alz(&["build"], Some("account-one")).code, 0);
        assert_eq!(second.alz(&["build"], Some("account-two")).code, 0);
        let (one, two) = (first.tree(), second.tree());
        assert!(one.keys().any(|path| path == "AGENTS.md"), "{one:?}");
        assert_eq!(one, two);
        assert_eq!(first.alz(&["check"], Some("account-two")).code, 0);
    }

    /// Spec §13 item 24: each declarer that may not declare fails the build with the layers
    /// named, and nothing is written.
    #[test]
    fn a_layer_that_may_not_declare_an_account_fails_the_build_and_names_it() {
        let cases: [(&str, &[&str], &[&str]); 4] = [
            ("a repo layer", &CLIENT_STACK, &["repo"]),
            (
                "a group layer",
                &["../personal", "../client", "../group", "./.layer"],
                &["group"],
            ),
            (
                "a public client layer",
                &["../personal", "../public-client", "./.layer"],
                &["public-client"],
            ),
            (
                "a second client layer",
                &["../personal", "../client", "../client-b", "./.layer"],
                &["client", "client-b"],
            ),
        ];
        for (what, stack, declaring) in cases {
            let world = World::new(stack, declaring);
            let before = world.tree();
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{what}: {}", run.stderr);
            let line = run
                .stderr
                .lines()
                .find(|line| line.starts_with("error [account]"))
                .unwrap_or_else(|| panic!("{what}: no error [account]: {}", run.stderr));
            for name in declaring {
                assert!(line.contains(name), "{what}: {line}");
            }
            assert_eq!(world.tree(), before, "{what}: nothing is written");
        }
    }

    /// Spec §13 item 24: the one allowed client layer outside the repository declares an
    /// account and the build succeeds.
    #[test]
    fn the_private_client_layer_outside_the_repository_may_declare_an_account() {
        let world = World::new(&CLIENT_STACK, &["client"]);
        let run = world.alz(&["build"], None);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(!run.stderr.contains("[account]"), "{}", run.stderr);
    }

    /// Spec §13 item 24: a layer that sets a protected variable through `settings.yaml` (also
    /// under a key such as `NAME=x` that is not a plain variable name), an MCP `env_from`, an
    /// agent's native `mcpServers`, or a passthrough settings, `.mcp.json` or `.toml` file, or
    /// that has a passthrough path with a non-ASCII character, fails the build, names the
    /// layer, and writes nothing.
    #[test]
    fn a_layer_that_sets_an_account_variable_fails_the_build_and_names_the_layer() {
        let cases: [(&str, &str, &str); 7] = [
            (
                "settings env",
                ".layer/settings.yaml",
                "claude:\n  env:\n    CODEX_HOME: /x\n",
            ),
            (
                "settings env key that is not a name",
                ".layer/settings.yaml",
                "claude:\n  env:\n    \"NAME=x\": y\n",
            ),
            (
                "mcp env_from",
                ".layer/mcp.yaml",
                "srv:\n  transport: stdio\n  command: x\n  env_from: [HOME]\n",
            ),
            (
                "agent native mcpServers",
                ".layer/agents/a.md",
                "---\nname: a\ndescription: d\nclaude:\n  mcpServers: {}\n---\nbody\n",
            ),
            (
                "passthrough .mcp.json",
                ".layer/targets/claude/.mcp.json",
                "{\"mcpServers\":{\"a\":{\"env\":{\"XDG_CONFIG_HOME\":\"x\"}}}}\n",
            ),
            (
                "passthrough toml",
                ".layer/targets/claude/extra/config.toml",
                "[shell_environment_policy]\nset = { HOME = \"x\" }\n",
            ),
            (
                "passthrough path with a non-ASCII character",
                ".layer/targets/claude/.claude/caf\u{e9}.md",
                "text\n",
            ),
        ];
        for (what, file, text) in cases {
            let world = World::new(&CLIENT_STACK, &[]);
            world
                .scratch
                .make_dirs(
                    Path::new(&format!("work/repo/{file}"))
                        .parent()
                        .expect("a parent"),
                )
                .expect("made");
            world
                .scratch
                .write(format!("work/repo/{file}"), text)
                .expect("written");
            let before = world.tree();
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{what}: {}", run.stderr);
            assert!(
                run.stderr
                    .lines()
                    .any(|line| line.starts_with("error [") && line.contains("repo")),
                "{what}: {}",
                run.stderr
            );
            assert_eq!(world.tree(), before, "{what}: nothing is written");
        }
    }

    /// Spec §13 item 24: a Codex `shell_environment_policy` whose `set` names an account
    /// variable, or whose `exclude` matches one, also under a profile, fails the build and names
    /// the layer; a policy that only changes `include_only` or `inherit` still builds.
    #[test]
    fn a_codex_policy_that_touches_an_account_variable_fails_and_one_that_does_not_builds() {
        let refused = [
            "codex:\n  shell_environment_policy:\n    set: { CODEX_HOME: /x }\n",
            "codex:\n  shell_environment_policy:\n    exclude: [\"HOME\"]\n",
            "codex:\n  profiles:\n    work:\n      shell_environment_policy:\n        set: { XDG_STATE_HOME: /x }\n",
        ];
        for text in refused {
            let world = World::new(&CLIENT_STACK, &[]);
            world
                .scratch
                .write("work/repo/.layer/settings.yaml", text)
                .expect("written");
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{text}: {}", run.stderr);
            assert!(
                run.stderr.contains("error [protected-variable]") && run.stderr.contains("repo"),
                "{text}: {}",
                run.stderr
            );
        }
        let allowed = World::new(&CLIENT_STACK, &[]);
        allowed
            .scratch
            .write(
                "work/repo/.layer/settings.yaml",
                "codex:\n  shell_environment_policy:\n    inherit: core\n    include_only: [\"PATH\"]\n    exclude: [\"SECRET_*\"]\n",
            )
            .expect("written");
        let run = allowed.alz(&["build"], None);
        assert_eq!(run.code, 0, "{}", run.stderr);
    }

    /// Spec §13 item 24: a `client` layer reached through a link inside the repository that
    /// declares `account` fails the build and names the layer, and nothing is written.
    #[test]
    fn a_client_layer_reached_through_a_link_inside_the_repository_may_not_declare_an_account() {
        let world = World::new(&["../personal", "./linked", "./.layer"], &["client"]);
        world
            .scratch
            .make_link("work/repo/linked", world.scratch.join("work/client"))
            .expect("linked");
        let before = world.tree();
        let run = world.alz(&["build"], None);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert!(
            run.stderr
                .lines()
                .any(|line| line.starts_with("error [account]") && line.contains("client")),
            "{}",
            run.stderr
        );
        assert_eq!(world.tree(), before, "nothing is written");
    }

    /// Spec §13 item 24: a Codex `shell_environment_policy` whose `filters` entry not set to
    /// `include` matches a protected variable fails the build and names the layer, also under a
    /// profile; an entry set to `include`, or one that matches no protected name, builds.
    #[test]
    fn a_codex_filters_entry_that_is_not_include_and_matches_an_account_variable_fails() {
        let refused = [
            "codex:\n  shell_environment_policy:\n    filters:\n      \"*home*\": exclude\n",
            "codex:\n  shell_environment_policy:\n    filters:\n      \"XDG_?????_HOME\": exclude\n",
            "codex:\n  profiles:\n    work:\n      shell_environment_policy:\n        filters:\n          \"CODEX_HOME\": exclude\n",
        ];
        for text in refused {
            let world = World::new(&CLIENT_STACK, &[]);
            world
                .scratch
                .write("work/repo/.layer/settings.yaml", text)
                .expect("written");
            let before = world.tree();
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{text}: {}", run.stderr);
            assert!(
                run.stderr.contains("error [protected-variable]") && run.stderr.contains("repo"),
                "{text}: {}",
                run.stderr
            );
            assert_eq!(world.tree(), before, "{text}: nothing is written");
        }
        let allowed = [
            "codex:\n  shell_environment_policy:\n    filters:\n      \"*home*\": include\n",
            "codex:\n  shell_environment_policy:\n    filters:\n      \"SECRET_*\": exclude\n",
        ];
        for text in allowed {
            let world = World::new(&CLIENT_STACK, &[]);
            world
                .scratch
                .write("work/repo/.layer/settings.yaml", text)
                .expect("written");
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 0, "{text}: {}", run.stderr);
        }
    }

    /// Spec §13 item 24 (§7.11): a passthrough settings file, `.mcp.json`, plugin manifest or
    /// agent that sets a protected variable fails the build and names the layer, at any depth
    /// and whatever its target; the same files without one build.
    #[test]
    fn a_passthrough_file_of_any_depth_or_target_that_sets_an_account_variable_fails() {
        const SETTINGS: &str = "{\"env\":{\"HOME\":\"x\"}}\n";
        const MCP: &str = "{\"mcpServers\":{\"a\":{\"env\":{\"XDG_STATE_HOME\":\"x\"}}}}\n";
        const AGENT: &str =
            "---\nname: a\nmcpServers:\n  s:\n    env:\n      CODEX_HOME: x\n---\nbody\n";
        let refused: [(&str, &str); 9] = [
            (".layer/targets/claude/.claude/settings.json", SETTINGS),
            (
                ".layer/targets/claude/.claude/settings.local.json",
                SETTINGS,
            ),
            (
                ".layer/targets/claude/a/b/c/.claude/settings.json",
                SETTINGS,
            ),
            (".layer/targets/claude/a/b/.mcp.json", MCP),
            (".layer/targets/claude/x/.claude-plugin/plugin.json", MCP),
            (".layer/targets/claude/.claude/agents/team/deep/a.md", AGENT),
            (".layer/targets/codex/.claude/settings.json", SETTINGS),
            (".layer/targets/codex/sub/.mcp.json", MCP),
            (".layer/targets/codex/.claude/agents/a.md", AGENT),
        ];
        for (file, text) in refused {
            let world = World::new(&CLIENT_STACK, &[]);
            world
                .scratch
                .make_dirs(
                    Path::new(&format!("work/repo/{file}"))
                        .parent()
                        .expect("a parent"),
                )
                .expect("made");
            world
                .scratch
                .write(format!("work/repo/{file}"), text)
                .expect("written");
            let before = world.tree();
            let run = world.alz(&["build"], None);
            assert_eq!(run.code, 1, "{file}: {}", run.stderr);
            assert!(
                run.stderr
                    .lines()
                    .any(|line| line.starts_with("error [protected-variable]")
                        && line.contains("repo")),
                "{file}: {}",
                run.stderr
            );
            assert_eq!(world.tree(), before, "{file}: nothing is written");
        }
        let world = World::new(&CLIENT_STACK, &[]);
        world
            .scratch
            .make_dirs("work/repo/.layer/targets/claude/.claude")
            .expect("made");
        world
            .scratch
            .write(
                "work/repo/.layer/targets/claude/.claude/settings.json",
                "{\"env\":{\"FOO\":\"x\"}}\n",
            )
            .expect("written");
        let run = world.alz(&["build"], None);
        assert_eq!(run.code, 0, "{}", run.stderr);
    }
}
