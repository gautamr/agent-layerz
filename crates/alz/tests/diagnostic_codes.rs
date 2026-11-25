//! The two code tables of spec §10.1, "Merge diagnostics" and "Build and command diagnostics"
//! (TASK-55.9.9): the codes the tests below reach are exactly the codes of the tables, less the
//! ones named in `NOT_REACHED_HERE` with the test that proves them. Each reachable code is
//! produced through the binary, with the level the table gives, on standard error, with the exit
//! code of §10.1 (1 for an error, 0 for a warning alone). The wording of a message is tested where
//! the message is made, and the quoted ones in `build_check.rs` and the other command tests.
//! Every run has an isolated `HOME`, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    /// The stack of a repository with an extra, more general layer `.layer-a`.
    const TWO_LAYERS: &str =
        "stack:\n  - source: path:./.layer-a\n  - source: path:./.layer\ntargets: [claude]\n";
    const BASE_LAYER: &str = "name: base\nkind: personal\nvisibility: private\n";
    const AGENT: &str = "---\nname: a\ndescription: d\ntools: [read]\n---\nbody\n";

    /// A repository whose build must produce the diagnostic, as files below the scratch folder
    /// (`repo/`, `home/`, `config/`, `state/`), the command and its extra environment.
    struct Case {
        code: &'static str,
        level: &'static str,
        files: &'static [(&'static str, &'static str)],
        arguments: &'static [&'static str],
        environment: &'static [(&'static str, &'static str)],
        /// Files written after a first `alz build`, which leaves its `layer.lock` behind them.
        after_build: &'static [(&'static str, &'static str)],
    }

    impl Case {
        /// 1 for an error. A warning or a note alone exits 0 (§10.1), except where the changed
        /// declaration withdraws the approval: the account then fails closed, which `alz account
        /// which` reports with exit 1 (§10.3).
        fn expected_exit(&self) -> i32 {
            let fails_closed = !self.after_build.is_empty();
            i32::from(self.level == "error" || fails_closed)
        }
    }

    const fn build(
        code: &'static str,
        level: &'static str,
        files: &'static [(&'static str, &'static str)],
    ) -> Case {
        Case {
            code,
            level,
            files,
            arguments: &["build"],
            environment: &[],
            after_build: &[],
        }
    }

    const fn running(
        code: &'static str,
        level: &'static str,
        arguments: &'static [&'static str],
        files: &'static [(&'static str, &'static str)],
    ) -> Case {
        Case {
            code,
            level,
            files,
            arguments,
            environment: &[],
            after_build: &[],
        }
    }

    /// An allowed repository whose client layer declares `client-a` and whose repo layer declares
    /// an account, which a repo layer may not. The repo layer was changed after the build, so the
    /// recorded declaration is the one allowed, and the folders of the account exist.
    fn allowed_client_case(code: &'static str) -> Case {
        Case {
            code,
            level: "warning",
            files: &[
                (
                    "repo/layer.yaml",
                    "stack:\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n",
                ),
                (
                    "client/layer.yaml",
                    "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
                ),
                (
                    "state/agent-layerz/allowed.yaml",
                    "- root: \"{real}/repo\"\n  account: { claude: client-a }\n  source: \"{real}/client\"\n",
                ),
                ("home/.claude-client-a/keep", ""),
                ("home/.codex-client-a/keep", ""),
            ],
            arguments: &["account", "which"],
            environment: &[],
            after_build: &[(
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\naccount: client-b\n",
            )],
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one table of cases, one entry for each code of the spec"
    )]
    fn cases() -> Vec<Case> {
        vec![
            build(
                "frontmatter",
                "error",
                &[("repo/.layer/agents/a.md", "---\nname: a\n")],
            ),
            build(
                "instructions",
                "error",
                &[(
                    "repo/.layer/instructions/00-core.md",
                    "---\nbogus: 1\n---\nbody\n",
                )],
            ),
            build(
                "agent",
                "error",
                &[(
                    "repo/.layer/agents/a.md",
                    "---\nname: a\ndescription: d\nmemory: weird\n---\nbody\n",
                )],
            ),
            build(
                "agent-duplicate",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    ("repo/.layer-a/agents/a.md", AGENT),
                    ("repo/.layer/agents/a.md", AGENT),
                ],
            ),
            build(
                "agent-overlay",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    ("repo/.layer-a/agents/a.md", AGENT),
                    (
                        "repo/.layer/agents/a.overlay.md",
                        "---\nmode: extend\n---\nmore\n",
                    ),
                ],
            ),
            build(
                "agent-replace",
                "warning",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    ("repo/.layer-a/agents/a.md", AGENT),
                    (
                        "repo/.layer/agents/a.overlay.md",
                        "---\nextends: a\nmode: replace\n---\nnew\n",
                    ),
                ],
            ),
            build(
                "tool-widen",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    ("repo/.layer-a/agents/a.md", AGENT),
                    (
                        "repo/.layer/agents/a.overlay.md",
                        "---\nextends: a\ntools: +[write]\n---\nmore\n",
                    ),
                ],
            ),
            build(
                "skill",
                "error",
                &[("repo/.layer/skills/s/notes.md", "no SKILL.md here\n")],
            ),
            build(
                "command",
                "error",
                &[(
                    "repo/.layer/commands/c.md",
                    "---\ndescription: d\nbogus: 1\n---\nbody\n",
                )],
            ),
            build(
                "skill-command-clash",
                "error",
                &[
                    (
                        "repo/.layer/skills/x/SKILL.md",
                        "---\nname: x\ndescription: d\n---\nbody\n",
                    ),
                    (
                        "repo/.layer/commands/x.md",
                        "---\ndescription: d\n---\nbody\n",
                    ),
                ],
            ),
            build(
                "slot",
                "error",
                &[(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\nrequires_slots: [Not_A_Skill]\n",
                )],
            ),
            build(
                "slot-missing",
                "warning",
                &[
                    (
                        "repo/.layer/layer.yaml",
                        "name: shop-api\nkind: repo\nvisibility: private\nrequires_slots: [glossary]\n",
                    ),
                    (
                        "repo/layer.yaml",
                        "stack:\n  - source: path:./.layer\ntargets: [claude]\noptions:\n  strict_slots: false\n",
                    ),
                ],
            ),
            build(
                "slot-command",
                "error",
                &[
                    (
                        "repo/.layer/agents/a.md",
                        "---\nname: a\ndescription: d\nskills: [c]\n---\nbody\n",
                    ),
                    (
                        "repo/.layer/commands/c.md",
                        "---\ndescription: d\n---\nbody\n",
                    ),
                ],
            ),
            build(
                "hooks",
                "error",
                &[(
                    "repo/.layer/hooks/hooks.yaml",
                    "- id: x\n  event: bogus\n  command: echo\n",
                )],
            ),
            build(
                "hook-locked",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    (
                        "repo/.layer-a/hooks/hooks.yaml",
                        "- id: x\n  event: stop\n  command: echo\n  locked: true\n",
                    ),
                    (
                        "repo/.layer/hooks/hooks.yaml",
                        "- id: x\n  disabled: true\n",
                    ),
                ],
            ),
            build(
                "permissions",
                "error",
                &[("repo/.layer/permissions.yaml", "bogus: [x]\n")],
            ),
            build(
                "permission-floor",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    ("repo/.layer-a/layer.yaml", BASE_LAYER),
                    ("repo/.layer-a/permissions.yaml", "deny: [\"read:.env*\"]\n"),
                    ("repo/.layer/permissions.yaml", "deny: \"-[read:.env*]\"\n"),
                ],
            ),
            build(
                "permission-shadowed",
                "warning",
                &[(
                    "repo/.layer/permissions.yaml",
                    "allow: [\"bash:ls\"]\ndeny: [\"bash:ls\"]\n",
                )],
            ),
            build(
                "mcp",
                "error",
                &[(
                    "repo/.layer/mcp.yaml",
                    "\"Bad Id\":\n  transport: http\n  url: https://example.invalid/mcp\n",
                )],
            ),
            build(
                "models",
                "error",
                &[(
                    "repo/.layer/models.yaml",
                    "aliases:\n  strong: { bogus: x }\n",
                )],
            ),
            build(
                "settings",
                "error",
                &[("repo/.layer/settings.yaml", "bogus: {}\n")],
            ),
            build(
                "protected-variable",
                "error",
                &[(
                    "repo/.layer/settings.yaml",
                    "claude:\n  env:\n    CLAUDE_CONFIG_DIR: /x\n",
                )],
            ),
            build(
                "rule",
                "error",
                &[("repo/.layer/rules/r.md", "---\nbogus: 1\n---\nbody\n")],
            ),
            build(
                "passthrough",
                "warning",
                &[("repo/.layer/targets/claude/extra.txt", "raw\n")],
            ),
            build(
                "unresolved",
                "error",
                &[(
                    "repo/.layer/agents/zz.overlay.md",
                    "---\nextends: zz\n---\nmore\n",
                )],
            ),
            build(
                "leak-forbid",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    (
                        "repo/.layer-a/layer.yaml",
                        "name: base\nkind: personal\nvisibility: public\nforbid_patterns:\n  - \"(?i)\\\\bacme\\\\b\"\n",
                    ),
                    ("repo/.layer-a/instructions/00-core.md", "Works at Acme.\n"),
                ],
            ),
            build(
                "leak-secret",
                "error",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    (
                        "repo/.layer-a/layer.yaml",
                        "name: base\nkind: personal\nvisibility: public\n",
                    ),
                    (
                        "repo/.layer-a/instructions/00-core.md",
                        "token ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
                    ),
                ],
            ),
            build(
                "leak-email",
                "warning",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    (
                        "repo/.layer-a/layer.yaml",
                        "name: base\nkind: personal\nvisibility: public\n",
                    ),
                    (
                        "repo/.layer-a/instructions/00-core.md",
                        "mail someone@private-company.test\n",
                    ),
                ],
            ),
            build(
                "leak-host",
                "warning",
                &[
                    ("repo/layer.yaml", TWO_LAYERS),
                    (
                        "repo/.layer-a/layer.yaml",
                        "name: base\nkind: personal\nvisibility: public\n",
                    ),
                    (
                        "repo/.layer-a/instructions/00-core.md",
                        "see git.corp.internal\n",
                    ),
                ],
            ),
            build(
                "collision",
                "error",
                &[("repo/AGENTS.md", "hand written\n")],
            ),
            running("check", "error", &["check"], &[]),
            build(
                "budget",
                "error",
                &[
                    (
                        "repo/layer.yaml",
                        "stack:\n  - source: path:./.layer\ntargets: [claude]\nbudgets:\n  instructions_max_lines: 1\n",
                    ),
                    (
                        "repo/.layer/instructions/00-core.md",
                        "one\ntwo\nthree\nfour\n",
                    ),
                ],
            ),
            build(
                "protected",
                "warning",
                &[(
                    "repo/.layer/targets/claude/.claude/agent-memory/x.md",
                    "memory\n",
                )],
            ),
            build(
                "layer",
                "error",
                &[("repo/.layer/instructions/a\u{1}b.md", "text\n")],
            ),
            build(
                "target",
                "error",
                &[(
                    "repo/layer.yaml",
                    "stack:\n  - source: path:./.layer\ntargets: [claude, codex]\n",
                )],
            ),
            build(
                "claude-agent",
                "error",
                &[(
                    "repo/.layer/agents/a.md",
                    "---\nname: a\ndescription: d\ntools: []\n---\nbody\n",
                )],
            ),
            build(
                "tool-scope",
                "warning",
                &[(
                    "repo/.layer/agents/a.md",
                    "---\nname: a\ndescription: d\ntools: [\"write:docs/**\"]\n---\nbody\n",
                )],
            ),
            build(
                "hook-on-fail",
                "warning",
                &[(
                    "repo/.layer/hooks/hooks.yaml",
                    "- id: x\n  event: stop\n  command: echo\n  on_fail: deny\n",
                )],
            ),
            running(
                "adopt",
                "error",
                &["build", "--adopt"],
                &[(
                    "repo/.claude/agents/bad.md",
                    "---\nname: bad\ndescription: x\npermissionMode: weird\n---\nbody\n",
                )],
            ),
            running("explain", "error", &["explain", "agent", "nothing"], &[]),
            Case {
                code: "ls",
                level: "error",
                files: &[],
                arguments: &["ls", "--live"],
                environment: &[("ALZ_CLAUDE_BIN", "{scratch}/no-such-claude")],
                after_build: &[],
            },
            running("init", "error", &["init"], &[]),
            build(
                "identity",
                "note",
                &[
                    (
                        "repo/layer.yaml",
                        "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
                    ),
                    (
                        "personal/layer.yaml",
                        "name: personal\nkind: personal\nvisibility: public\ncommit_identity:\n  name: Owner Example\n  email: owner@example.com\n",
                    ),
                ],
            ),
            build(
                "account",
                "error",
                &[(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\naccount: client-a\n",
                )],
            ),
            allowed_client_case("account-declaration-ignored"),
            allowed_client_case("account-changed"),
            running(
                "account-file-in-work-tree",
                "warning",
                &["account", "which"],
                &[
                    (".git/HEAD", "ref: refs/heads/main\n"),
                    (
                        "config/agent-layerz/accounts.yaml",
                        "default_name: personal\n",
                    ),
                ],
            ),
            running(
                "account-file-ignored",
                "warning",
                &["account", "which"],
                &[("config/agent-layerz/accounts.yaml", "a: [\n")],
            ),
            running(
                "account-config-shape",
                "warning",
                &["account", "which"],
                &[("config/agent-layerz/accounts.yaml", "- a\n")],
            ),
            running(
                "account-config-key",
                "warning",
                &["account", "which"],
                &[("config/agent-layerz/accounts.yaml", "bogus: 1\n")],
            ),
            running(
                "account-config-default-name",
                "warning",
                &["account", "which"],
                &[(
                    "config/agent-layerz/accounts.yaml",
                    "default_name: Bad Name\n",
                )],
            ),
            running(
                "account-config-folder",
                "warning",
                &["account", "which"],
                &[(
                    "config/agent-layerz/accounts.yaml",
                    "folders:\n  relative/path: work\n",
                )],
            ),
            running(
                "account-allowed-shape",
                "warning",
                &["account", "which"],
                &[("state/agent-layerz/allowed.yaml", "a: 1\n")],
            ),
            running(
                "account-allowed-entry",
                "warning",
                &["account", "which"],
                &[("state/agent-layerz/allowed.yaml", "- 5\n")],
            ),
            Case {
                code: "account-foreign",
                level: "warning",
                files: &[],
                arguments: &["account", "which"],
                environment: &[("CLAUDE_CONFIG_DIR", "{scratch}/elsewhere")],
                after_build: &[],
            },
            Case {
                code: "account-home",
                level: "warning",
                files: &[],
                arguments: &["account", "which"],
                environment: &[("HOME", "relative-home")],
                after_build: &[],
            },
            Case {
                code: "account-xdg-relative",
                level: "warning",
                files: &[],
                arguments: &["account", "which"],
                environment: &[("XDG_CONFIG_HOME", "relative-config")],
                after_build: &[],
            },
        ]
    }

    /// Codes of the two tables that this file does not reach, each with the test that proves it.
    const NOT_REACHED_HERE: &[(&str, &str)] = &[
        (
            "write",
            "crates/alz/tests/write_error.rs::a_folder_that_cannot_be_listed_is_the_error_write_naming_it_and_nothing_is_written: it needs a folder with mode 0 (not as the superuser)",
        ),
        (
            "claude",
            "crates/alz/tests/doctor_live.rs: it needs a stand-in claude that prints on its standard error",
        ),
        (
            "account-allow",
            "crates/alz/tests/account_allow.rs and allow_hint.rs: it needs a terminal",
        ),
        (
            "account-default-name-folder",
            "crates/alz-core/src/account/resolution/tests.rs::a_default_name_that_is_also_an_account_folder_warns_with_the_spec_text",
        ),
        (
            "account-config-duplicate",
            "crates/alz-io/tests/account_files.rs: two folders keys that resolve to one directory",
        ),
    ];

    /// The codes of the first column of the two tables of §10.1.
    fn codes_of_the_spec() -> BTreeSet<String> {
        let spec = include_str!("../../../docs/LAYERING-SPEC.md");
        let start = spec
            .find("**Merge diagnostics**")
            .expect("the merge table starts");
        let end = spec.find("**Exit codes.**").expect("the exit codes follow");
        spec.get(start..end)
            .expect("a slice")
            .lines()
            .filter(|line| line.starts_with("| `"))
            .filter_map(|line| line.split('|').nth(1))
            .flat_map(|first| {
                first
                    .split(',')
                    .map(|code| code.trim().trim_matches('`').to_owned())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Spec §10.1: the cases and the exemptions cover the codes of both tables, no more, no less.
    #[test]
    fn every_code_of_the_two_tables_has_a_case_or_a_named_test_and_no_other_code_does() {
        let spec = codes_of_the_spec();
        assert!(spec.len() > 60, "{} codes read", spec.len());
        let handled: BTreeSet<String> = cases()
            .iter()
            .map(|case| case.code.to_owned())
            .chain(NOT_REACHED_HERE.iter().map(|(code, _)| (*code).to_owned()))
            .collect();
        let missing: Vec<_> = spec.difference(&handled).collect();
        let unknown: Vec<_> = handled.difference(&spec).collect();
        assert_eq!(
            (missing, unknown),
            (vec![], vec![]),
            "codes of the table with no case, then cases with no code in the table"
        );
        let reached: BTreeSet<_> = cases().iter().map(|case| case.code).collect();
        assert!(
            NOT_REACHED_HERE
                .iter()
                .all(|(code, _)| !reached.contains(code))
        );
    }

    /// Spec §10.1: each code is printed through the binary as `<level> [<code>] …` on standard
    /// error, standard output holds no diagnostic, and the exit code is 1 when the diagnostic is
    /// an error and 0 when it is a warning or a note alone.
    #[test]
    fn each_code_of_the_tables_is_printed_with_its_level_and_the_exit_code_of_that_level() {
        let problems: Vec<String> = cases().iter().filter_map(problem_of).collect();
        assert_eq!(problems, Vec::<String>::new());
    }

    fn problem_of(case: &Case) -> Option<String> {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["home", "config", "state", "cache"] {
            scratch.make_dirs(folder).expect("made");
        }
        let base = [
            (
                "repo/layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            (
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
            ("repo/.git/HEAD", "ref: refs/heads/main\n"),
        ];
        let real = scratch
            .real_path()
            .expect("a real path")
            .display()
            .to_string();
        let put = |files: &[(&str, &str)]| {
            for (path, text) in files {
                if let Some(parent) = Path::new(path).parent() {
                    scratch.make_dirs(parent).expect("made");
                }
                scratch
                    .write(path, text.replace("{real}", &real))
                    .expect("written");
            }
        };
        let command_in = |arguments: &[&str]| {
            let mut command: Command = cargo_bin_cmd!("alz");
            command
                .current_dir(scratch.join("repo"))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", scratch.join("home"))
                .env("XDG_CONFIG_HOME", scratch.join("config"))
                .env("XDG_STATE_HOME", scratch.join("state"))
                .env("XDG_CACHE_HOME", scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(arguments);
            command
        };
        put(&base);
        put(case.files);
        if !case.after_build.is_empty() {
            let first = command_in(&["build"])
                .output()
                .expect("the first build runs");
            assert_eq!(
                first.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&first.stderr)
            );
            put(case.after_build);
        }
        let mut command = command_in(case.arguments);
        for (name, value) in case.environment {
            command.env(
                name,
                value.replace("{scratch}", &scratch.path().display().to_string()),
            );
        }
        let output = command.output().expect("alz runs and ends in time");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let prefix = format!("{} [{}]", case.level, case.code);
        let found = stderr.lines().any(|line| line.starts_with(&prefix));
        let code = output.status.code();
        let expected = Some(case.expected_exit());
        let diagnostic_on_stdout = stdout.lines().any(|line| line.starts_with(&prefix));
        (!found || diagnostic_on_stdout || code != expected).then(|| {
            format!(
                "{}: found {found}, on stdout {diagnostic_on_stdout}, exit {code:?}, wanted {expected:?}\n{stderr}{stdout}",
                case.code
            )
        })
    }
}
