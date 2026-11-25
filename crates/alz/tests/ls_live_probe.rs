//! `alz ls --live` against a fake `claude` that records what it was given (spec §10.3 "`alz ls
//! [--live]`", §13 item 12, TASK-55.9): the arguments, where the program is looked for, which
//! manifests may ask for it, the lines of the report, and the order of hooks and servers. The
//! time limit and the output caps have no seam in the binary; `alz-io` tests them where their
//! length can be set (`crates/alz-io/tests/tool_runner.rs`, `crates/alz-io/src/tool/environment.rs`).
//! Every run has its own `HOME`, so nothing reads the real `~/.claude*`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, copy_tree};
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

    const INIT: &str = r#"{"type":"system","subtype":"init","agents":["architect","Explore"],"skills":[],"mcp_servers":[]}"#;

    /// A fake `claude` that writes what it saw to `record`: its arguments one per line, its working
    /// directory, whether standard input was at its end, and a name that says which copy ran. It
    /// uses only shell built-ins, because its `PATH` holds nothing but absolute entries.
    fn recording_claude(name: &str, record: &Path, init: &str) -> String {
        format!(
            "#!/bin/sh\n{{\nprintf 'ran=%s\\n' '{name}'\nprintf 'cwd=%s\\n' \"$(pwd)\"\nif read -r _line; then echo 'stdin=data'; else echo 'stdin=eof'; fi\nprintf 'count=%s\\n' \"$#\"\nfor argument in \"$@\"; do printf 'arg=%s\\n' \"$argument\"; done\n}} > '{record}'\nprintf '%s\\n' '{init}'\n",
            record = record.display()
        )
    }

    impl Workspace {
        fn of(fixture: &str, repository: &str) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            copy_tree(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures")
                    .join(fixture),
                &work,
            )
            .expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            Self {
                repository: work.join(repository),
                scratch,
            }
        }

        /// A repository of two layers: `personal/` and the repository's own `.layer/`.
        fn with_layers(
            manifest: &str,
            personal: &[(&str, &str)],
            repository: &[(&str, &str)],
        ) -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            scratch.make_dirs("home").expect("made");
            scratch.make_dirs("personal").expect("made");
            scratch.make_dirs("repo/.layer").expect("made");
            scratch.write("repo/layer.yaml", manifest).expect("written");
            scratch
                .write(
                    "personal/layer.yaml",
                    "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
                )
                .expect("written");
            scratch
                .write(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
            for (path, text) in personal {
                if let Some(parent) = Path::new(path).parent() {
                    scratch
                        .make_dirs(Path::new("personal").join(parent))
                        .expect("made");
                }
                scratch
                    .write(format!("personal/{path}"), text)
                    .expect("written");
            }
            for (path, text) in repository {
                if let Some(parent) = Path::new(path).parent() {
                    scratch
                        .make_dirs(Path::new("repo/.layer").join(parent))
                        .expect("made");
                }
                scratch
                    .write(format!("repo/.layer/{path}"), text)
                    .expect("written");
            }
            Self {
                repository: scratch.join("repo"),
                scratch,
            }
        }

        fn script(&self, name: &str, script: &str) -> PathBuf {
            self.scratch
                .write_executable(name, script)
                .expect("written");
            self.scratch.join(name)
        }

        /// `alz <arguments>` with `PATH` and `ALZ_CLAUDE_BIN` as given, and the other variables
        /// the program may receive set to values of the scratch directory.
        fn alz(
            &self,
            arguments: &[&str],
            path: Option<&OsStr>,
            claude_bin: Option<&OsStr>,
            config_dir: Option<&Path>,
        ) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(&self.repository)
                .env_remove("CLAUDE_CONFIG_DIR")
                .env_remove("ALZ_CLAUDE_BIN")
                .env_remove("CODEX_HOME")
                .env("PATH", path.unwrap_or_else(|| OsStr::new("/usr/bin:/bin")))
                .env("HOME", self.scratch.join("home"))
                .env("XDG_CACHE_HOME", self.scratch.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .timeout(Duration::from_secs(60))
                .args(arguments);
            if let Some(bin) = claude_bin {
                alz.env("ALZ_CLAUDE_BIN", bin);
            }
            if let Some(dir) = config_dir {
                alz.env("CLAUDE_CONFIG_DIR", dir);
            }
            let output = alz.output().expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn live(&self, claude: &Path) -> Run {
            self.alz(&["ls", "--live"], None, Some(claude.as_os_str()), None)
        }
    }

    /// Spec §10.3 `alz ls --live`: the program is run once, in the repository, with exactly these
    /// arguments, and without standard input.
    #[test]
    fn ls_live_runs_the_program_once_in_the_repository_with_the_spec_arguments() {
        let workspace = Workspace::of("22-ls", "repo");
        let record = workspace.scratch.join("record");
        let claude = workspace.script("fake-claude", &recording_claude("one", &record, INIT));
        let run = workspace.live(&claude);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        let real = workspace
            .scratch
            .real_path()
            .expect("real path")
            .join("work/repo");
        let seen = workspace.scratch.read_text("record").expect("it ran");
        let expected = format!(
            "ran=one\ncwd={}\nstdin=eof\ncount=14\narg=-p\narg=Reply with OK.\narg=--output-format\narg=stream-json\narg=--verbose\narg=--max-turns\narg=1\narg=--settings\narg={{\"disableAllHooks\":true}}\narg=--strict-mcp-config\narg=--mcp-config\narg={{\"mcpServers\":{{}}}}\narg=--tools\narg=\n",
            real.display()
        );
        assert_eq!(seen, expected);
    }

    /// Spec §10.3 `alz ls --live`: without `ALZ_CLAUDE_BIN` the name `claude` is looked up in the
    /// absolute entries of `PATH`, in order, and a relative or empty entry names nothing, so a
    /// program in the repository is never run.
    #[test]
    fn ls_live_finds_claude_in_the_absolute_entries_of_path_in_order_and_never_in_the_repository() {
        let workspace = Workspace::of("22-ls", "repo");
        let planted = workspace.scratch.join("work/repo/planted-ran");
        workspace.scratch.make_dirs("work/repo/bin").expect("made");
        workspace
            .scratch
            .write_executable(
                "work/repo/bin/claude",
                format!(
                    "#!/bin/sh\n: > '{}'\nprintf '%s\\n' '{INIT}'\n",
                    planted.display()
                ),
            )
            .expect("written");
        for directory in ["first", "second", "notexec"] {
            workspace.scratch.make_dirs(directory).expect("made");
        }
        // A regular file without an execute bit does not count, whatever comes before.
        workspace
            .scratch
            .write("notexec/claude", "#!/bin/sh\n")
            .expect("written");
        for name in ["first", "second"] {
            let record = workspace.scratch.join(format!("{name}-record"));
            workspace
                .scratch
                .write_executable(
                    format!("{name}/claude"),
                    recording_claude(name, &record, INIT),
                )
                .expect("written");
        }
        let path = format!(
            "bin:.::{}:{}:{}:/usr/bin:/bin",
            workspace.scratch.join("notexec").display(),
            workspace.scratch.join("first").display(),
            workspace.scratch.join("second").display()
        );
        let run = workspace.alz(&["ls", "--live"], Some(OsStr::new(&path)), None, None);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            workspace
                .scratch
                .read_text("first-record")
                .expect("the first ran")
                .starts_with("ran=first\n")
        );
        assert!(!workspace.scratch.exists("second-record"));
        assert!(
            !workspace.scratch.exists("work/repo/planted-ran"),
            "the repository's bin/claude ran"
        );
        assert!(
            run.stdout
                .contains("asking claude what it loads here (one short model call)…\n")
        );
    }

    /// Spec §10.3 `alz ls --live`: when the only entries that could hold `claude` are relative or
    /// empty, the program is not found and nothing in the repository runs.
    #[test]
    fn ls_live_with_only_relative_path_entries_does_not_find_claude() {
        let workspace = Workspace::of("22-ls", "repo");
        let planted = workspace.scratch.join("work/repo/planted-ran");
        workspace.scratch.make_dirs("work/repo/bin").expect("made");
        for relative in ["bin/claude", "claude"] {
            workspace
                .scratch
                .write_executable(
                    format!("work/repo/{relative}"),
                    format!(
                        "#!/bin/sh\n: > '{}'\nprintf '%s\\n' '{INIT}'\n",
                        planted.display()
                    ),
                )
                .expect("written");
        }
        let run = workspace.alz(
            &["ls", "--live"],
            Some(OsStr::new("bin:.::relative")),
            None,
            None,
        );
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr
                .contains("error [ls] could not run claude: not found\n"),
            "{}",
            run.stderr
        );
        assert!(
            !workspace.scratch.exists("work/repo/planted-ran"),
            "a program of the repository ran"
        );
    }

    /// Spec §10.3 `alz ls --live`: a relative `ALZ_CLAUDE_BIN` names nothing, though a file of
    /// that name is in the repository and executable.
    #[test]
    fn ls_live_refuses_a_relative_alz_claude_bin() {
        let workspace = Workspace::of("22-ls", "repo");
        let planted = workspace.scratch.join("work/repo/planted-ran");
        workspace.scratch.make_dirs("work/repo/bin").expect("made");
        workspace
            .scratch
            .write_executable(
                "work/repo/bin/claude",
                format!(
                    "#!/bin/sh\n: > '{}'\nprintf '%s\\n' '{INIT}'\n",
                    planted.display()
                ),
            )
            .expect("written");
        for relative in ["bin/claude", "./bin/claude", "../repo/bin/claude"] {
            let run = workspace.alz(&["ls", "--live"], None, Some(OsStr::new(relative)), None);
            assert_eq!(run.code, 1, "{relative}: {}{}", run.stdout, run.stderr);
            assert!(
                run.stderr
                    .contains(&format!("error [ls] could not run {relative}: not found\n")),
                "{relative}: {}",
                run.stderr
            );
            assert!(
                !workspace.scratch.exists("work/repo/planted-ran"),
                "{relative}: a program of the repository ran"
            );
        }
    }

    /// Spec §10.3 `alz ls --live`: an `ALZ_CLAUDE_BIN` without `/` is a name looked up in the
    /// absolute entries of `PATH`, as the default `claude` is.
    #[test]
    fn ls_live_looks_a_bare_alz_claude_bin_name_up_on_path() {
        let workspace = Workspace::of("22-ls", "repo");
        workspace.scratch.make_dirs("tools").expect("made");
        let record = workspace.scratch.join("record");
        workspace
            .scratch
            .write_executable("tools/mine", recording_claude("mine", &record, INIT))
            .expect("written");
        let path = format!(
            "{}:/usr/bin:/bin",
            workspace.scratch.join("tools").display()
        );
        let run = workspace.alz(
            &["ls", "--live"],
            Some(OsStr::new(&path)),
            Some(OsStr::new("mine")),
            None,
        );
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .contains("asking mine what it loads here (one short model call)…\n"),
            "{}",
            run.stdout
        );
        assert!(
            workspace
                .scratch
                .read_text("record")
                .expect("it ran")
                .starts_with("ran=mine\n")
        );
    }

    /// Spec §10.3 `alz ls --live`: a program that cannot be started is
    /// `error [ls] could not run <bin>: cannot start it`.
    #[test]
    fn ls_live_says_it_cannot_start_a_program_whose_interpreter_is_missing() {
        let workspace = Workspace::of("22-ls", "repo");
        let claude = workspace.script("fake-claude", "#!/no/such/interpreter\n");
        let run = workspace.live(&claude);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr.contains(&format!(
                "error [ls] could not run {}: cannot start it\n",
                claude.display()
            )),
            "{}",
            run.stderr
        );
    }

    /// Spec §10.3 `alz ls --live`: a built skill that Claude did not load is
    /// `error [ls] skill '<name>' was built but Claude did not load it`, and exit 1.
    #[test]
    fn ls_live_fails_naming_the_skill_claude_did_not_load() {
        let workspace = Workspace::with_layers(
            "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
            &[(
                "skills/adr/SKILL.md",
                "---\nname: adr\ndescription: Records decisions\n---\nText.\n",
            )],
            &[],
        );
        let claude = workspace.script(
            "fake-claude",
            &recording_claude(
                "one",
                &workspace.scratch.join("record"),
                r#"{"type":"system","subtype":"init","agents":[],"skills":["other"],"mcp_servers":[]}"#,
            ),
        );
        let run = workspace.live(&claude);
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stderr
                .contains("error [ls] skill 'adr' was built but Claude did not load it\n"),
            "{}",
            run.stderr
        );
        assert!(
            run.stdout
                .ends_with("ls --live: Claude does not match the build\n"),
            "{}",
            run.stdout
        );
    }

    /// Spec §10.3 `alz ls --live`: names that hold `:` (plugin skills) are left out of the
    /// `not from layers, skills` line, and the line is left out when nothing else remains; the
    /// counts still include them.
    #[test]
    fn ls_live_leaves_plugin_skill_names_with_a_colon_out_of_the_not_from_layers_line() {
        let workspace = Workspace::of("22-ls", "repo");
        let init = r#"{"type":"system","subtype":"init","agents":["architect"],"skills":["extra","plugin:one","plugin:two"],"mcp_servers":[]}"#;
        let claude = workspace.script(
            "fake-claude",
            &recording_claude("one", &workspace.scratch.join("record"), init),
        );
        let run = workspace.live(&claude);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout.contains(
                "claude loaded 1 agents, 3 skills and commands\n  not from layers, skills: extra\n"
            ),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("plugin:"), "{}", run.stdout);
        assert!(
            !run.stdout.contains("not from layers, agents"),
            "{}",
            run.stdout
        );

        let only_plugins = r#"{"type":"system","subtype":"init","agents":["architect"],"skills":["plugin:one"],"mcp_servers":[]}"#;
        let claude = workspace.script(
            "fake-claude",
            &recording_claude("two", &workspace.scratch.join("record"), only_plugins),
        );
        let run = workspace.live(&claude);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(!run.stdout.contains("not from layers"), "{}", run.stdout);
        assert!(
            run.stdout
                .ends_with("claude loaded 1 agents, 1 skills and commands\nls --live: Claude matches the build\n"),
            "{}",
            run.stdout
        );
    }

    /// Spec §10.3 `alz ls --live`: at most 4 MiB of standard output are read and the rest is
    /// dropped, so a session that prints its init event and then more than that still passes.
    #[test]
    fn ls_live_reads_the_init_event_of_a_program_that_goes_on_printing_past_the_cap() {
        let workspace = Workspace::of("22-ls", "repo");
        let claude = workspace.script(
            "fake-claude",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' '{INIT}'\nhead -c 6000000 /dev/zero | tr '\\0' 'x'\necho\nhead -c 300000 /dev/zero | tr '\\0' 'e' >&2\n"
            ),
        );
        let run = workspace.live(&claude);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout
                .ends_with("ls --live: Claude matches the build\n"),
            "{}",
            run.stdout.len()
        );
        // The 6 MB line is cut at 4 MiB and shown as a warning, with the 64 KiB of standard error
        // that were kept; nothing of the rest is read.
        let kept = 4 * 1024 * 1024;
        assert!(run.stderr.len() > kept, "{}", run.stderr.len());
        assert!(run.stderr.len() < kept + 70 * 1024, "{}", run.stderr.len());
    }

    const HOOKS_PERSONAL: &str = "- id: zeta\n  event: pre_tool\n  command: ./z.sh\n- id: alpha\n  event: stop\n  command: ./a.sh\n";
    const HOOKS_REPO: &str = "- id: beta\n  event: stop\n  command: ./b.sh\n";
    const SERVERS: &str = "alpha-server:\n  transport: stdio\n  command: npx\nmid-server:\n  transport: stdio\n  command: npx\nzulu-server:\n  transport: stdio\n  command: npx\nnot-enabled:\n  transport: stdio\n  command: npx\n";

    /// Spec §10.3 `alz ls`: hooks are listed in stack and file order, not in name order, and
    /// servers in `mcp.enable` order, not in name order; a server that is not enabled is absent.
    #[test]
    fn ls_lists_hooks_in_stack_order_and_servers_in_enable_order() {
        let workspace = Workspace::with_layers(
            "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\nmcp:\n  enable: [zulu-server, alpha-server, mid-server]\n",
            &[("hooks/hooks.yaml", HOOKS_PERSONAL), ("mcp.yaml", SERVERS)],
            &[("hooks/hooks.yaml", HOOKS_REPO)],
        );
        let run = workspace.alz(&["ls"], None, None, None);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        let line = |name: &str, layer: &str| format!("  {name:<28} {layer}\n");
        assert!(
            run.stdout.contains(&format!(
                "hooks (3)\n{}{}{}mcp (3)\n{}{}{}",
                line("zeta", "personal"),
                line("alpha", "personal"),
                line("beta", "shop-api"),
                line("zulu-server", "personal"),
                line("alpha-server", "personal"),
                line("mid-server", "personal"),
            )),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("not-enabled"), "{}", run.stdout);
    }

    /// Spec §13 item 12: with `CLAUDE_CONFIG_DIR` at two different empty account folders, the
    /// agents and skills that Claude lists, asked through `ls --live`, are identical, and the
    /// program saw the folder it was given.
    #[test]
    fn ls_live_gives_the_same_report_under_two_different_empty_account_folders() {
        let reports: Vec<String> = ["account-a", "account-b"]
            .iter()
            .map(|account| {
                let workspace = Workspace::of("22-ls", "repo");
                workspace.scratch.make_dir(account).expect("made");
                let folder = workspace.scratch.join(account);
                let listing = format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$CLAUDE_CONFIG_DIR\" > '{seen}'\nagents='\"architect\"'\nfor file in \"$CLAUDE_CONFIG_DIR\"/agents/*.md; do [ -e \"$file\" ] && agents=\"$agents,\\\"${{file##*/}}\\\"\"; done\nskills=''\nfor file in \"$CLAUDE_CONFIG_DIR\"/skills/*; do [ -e \"$file\" ] && skills=\"$skills\\\"${{file##*/}}\\\",\"; done\nprintf '{{\"type\":\"system\",\"subtype\":\"init\",\"agents\":[%s],\"skills\":[%s\"review\"],\"mcp_servers\":[]}}\\n' \"$agents\" \"$skills\"\n",
                    seen = workspace.scratch.join("seen").display()
                );
                let claude = workspace.script("fake-claude", &listing);
                let run = workspace.alz(
                    &["ls", "--live"],
                    None,
                    Some(claude.as_os_str()),
                    Some(&folder),
                );
                assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
                assert_eq!(
                    workspace.scratch.read_text("seen").expect("it ran").trim_end(),
                    folder.to_string_lossy(),
                    "the program saw the account folder"
                );
                run.stdout
                    .replace(workspace.scratch.path().to_string_lossy().as_ref(), "<s>")
            })
            .collect();
        assert_eq!(reports.first(), reports.get(1));
        assert!(
            reports.first().is_some_and(
                |report| report.contains("claude loaded 1 agents, 1 skills and commands\n")
            ),
            "{reports:?}"
        );
    }
}
