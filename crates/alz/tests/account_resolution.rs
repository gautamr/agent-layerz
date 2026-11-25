//! Spec §13 item 19, "Account resolution" (TASK-50.10), through `alz account which`
//! (TASK-55.9.9): each source per target, the `folders` entry that matches whole path segments
//! with the longest one winning, a symlinked path, a missing named account that fails closed
//! beside a missing default that does not, the user config that is ignored (inside the
//! repository, writable by group or others, under a writable parent), `allowed.yaml` entries
//! with a relative root or a bad account name, mise and direnv, a `layer.yaml` that stops the
//! walk, and a build that is byte-identical with and without account settings. Each run has its
//! own home, `XDG_*` folders and `PATH`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct World {
        scratch: ScratchDir,
        root: PathBuf,
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            for folder in [
                "home",
                "config/agent-layerz",
                "state/agent-layerz",
                "cache",
                "repo/.layer",
                "repo/.git",
                "repo/nested/sub",
            ] {
                scratch.make_dirs(folder).expect("made");
            }
            scratch
                .write("repo/.git/HEAD", "ref: refs/heads/main\n")
                .expect("written");
            scratch
                .write(
                    "repo/layer.yaml",
                    "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
                )
                .expect("written");
            scratch
                .write(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
            scratch
                .write(
                    "repo/nested/layer.yaml",
                    "stack:\n  - source: path:../.layer\ntargets: [claude]\n",
                )
                .expect("written");
            Self { scratch, root }
        }

        fn accounts(&self, text: &str) {
            let text = text.replace("{root}", &self.root.display().to_string());
            self.scratch
                .write("config/agent-layerz/accounts.yaml", text)
                .expect("written");
            self.scratch
                .set_mode("config/agent-layerz/accounts.yaml", 0o600)
                .expect("a mode");
        }

        fn account_folders(&self, names: &[&str]) {
            for name in names {
                self.scratch
                    .make_dirs(format!("home/.claude-{name}"))
                    .expect("made");
                self.scratch
                    .make_dirs(format!("home/.codex-{name}"))
                    .expect("made");
            }
        }

        fn run_in(&self, directory: &str, arguments: &[&str], extra: &[(&str, String)]) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            alz.current_dir(self.root.join(directory))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.root.join("home"))
                .env("XDG_CONFIG_HOME", self.root.join("config"))
                .env("XDG_STATE_HOME", self.root.join("state"))
                .env("XDG_CACHE_HOME", self.root.join("cache"))
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
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

        fn which(&self, directory: &str) -> Run {
            self.run_in(directory, &["account", "which"], &[])
        }
    }

    /// Spec §13 item 19: `--use` decides the account, and the line names the source `use`.
    #[test]
    fn the_use_choice_is_reported_with_the_source_use_and_its_folder() {
        let world = World::new();
        world.account_folders(&["work"]);
        let run = world.run_in("repo", &["account", "which", "--use", "claude=work"], &[]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .contains("claude: work (source: use) CLAUDE_CONFIG_DIR=~/.claude-work"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("codex: default (source: default)"),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 19: a value set outside alz is the source `environment` and is kept.
    #[test]
    fn a_value_set_outside_alz_is_the_source_environment() {
        let world = World::new();
        let run = world.run_in(
            "repo",
            &["account", "which"],
            &[("CLAUDE_CONFIG_DIR", "/somewhere/else".to_owned())],
        );
        assert!(
            run.stdout
                .contains("claude: set outside alz (source: environment) CLAUDE_CONFIG_DIR=\"/somewhere/else\" kept"),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 19: a `folders` entry of the user config decides, with the source `user
    /// config`; it matches whole path segments, and the longest entry wins.
    #[test]
    fn a_folders_entry_matches_whole_segments_and_the_longest_wins() {
        let world = World::new();
        world.account_folders(&["outer", "inner", "prefix"]);
        world.accounts(
            "folders:\n  \"{root}/repo\": outer\n  \"{root}/repo/nested\": inner\n  \"{root}/rep\": prefix\n",
        );
        let outer = world.which("repo");
        assert!(
            outer.stdout.contains("claude: outer (source: user config)"),
            "{}",
            outer.stdout
        );
        for directory in ["repo/nested", "repo/nested/sub"] {
            let inner = world.which(directory);
            assert!(
                inner.stdout.contains("claude: inner (source: user config)"),
                "{directory}: {}",
                inner.stdout
            );
        }
        assert!(!outer.stdout.contains("prefix"), "{}", outer.stdout);
    }

    /// Spec §13 item 19: a symlinked working directory resolves to the real folder.
    #[test]
    fn a_symlinked_working_directory_resolves_to_the_real_folder() {
        let world = World::new();
        world.account_folders(&["inner"]);
        world.accounts("folders:\n  \"{root}/repo/nested\": inner\n");
        world
            .scratch
            .make_link("link", world.root.join("repo/nested/sub"))
            .expect("a link");
        let run = world.which("link");
        assert!(
            run.stdout.contains("claude: inner (source: user config)"),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 19: a missing named account folder fails closed (exit 1), while a missing
    /// default folder never fails.
    #[test]
    fn a_missing_named_folder_fails_closed_and_a_missing_default_folder_does_not() {
        let world = World::new();
        let named = world.run_in("repo", &["account", "which", "--use", "claude=ghost"], &[]);
        assert_eq!(named.code, 1, "{}", named.stderr);
        assert!(
            named.stdout.contains("claude: fails closed (source: use)"),
            "{}",
            named.stdout
        );
        world.accounts("default_name: personal\n");
        let default = world.which("repo");
        assert_eq!(default.code, 0, "{}", default.stderr);
        assert!(
            default
                .stdout
                .contains("claude: default, called personal here (source: default)"),
            "{}",
            default.stdout
        );
    }

    /// Spec §13 item 19: a user config inside the repository, one that group or others can
    /// write, and one under a parent that group or others can write are each ignored with the
    /// reason, and the folders entry does not apply.
    #[test]
    fn an_unsafe_user_config_is_ignored_with_its_reason() {
        let inside = World::new();
        inside.account_folders(&["inner"]);
        inside
            .scratch
            .make_dirs("repo/cfg/agent-layerz")
            .expect("made");
        inside
            .scratch
            .write(
                "repo/cfg/agent-layerz/accounts.yaml",
                format!("folders:\n  \"{}/repo\": inner\n", inside.root.display()),
            )
            .expect("written");
        let run = inside.run_in(
            "repo",
            &["account", "which"],
            &[(
                "XDG_CONFIG_HOME",
                inside.root.join("repo/cfg").display().to_string(),
            )],
        );
        assert!(
            run.stdout
                .contains("(ignored: it lies inside the current repository)"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("claude: default (source: default)"),
            "{}",
            run.stdout
        );

        let writable = World::new();
        writable.account_folders(&["inner"]);
        writable.accounts("folders:\n  \"{root}/repo\": inner\n");
        writable
            .scratch
            .set_mode("config/agent-layerz/accounts.yaml", 0o666)
            .expect("a mode");
        let run = writable.which("repo");
        assert!(
            run.stdout
                .contains("(ignored: group or others can write it)"),
            "{}",
            run.stdout
        );

        let parent = World::new();
        parent.account_folders(&["inner"]);
        parent.accounts("folders:\n  \"{root}/repo\": inner\n");
        parent
            .scratch
            .set_mode("config/agent-layerz", 0o777)
            .expect("a mode");
        let run = parent.which("repo");
        assert!(
            run.stdout.contains("can be written by group or others)"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("claude: default (source: default)"),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 19: an `allowed.yaml` entry with a relative root, or an account name that
    /// breaks the rule, is ignored with a warning that says why.
    #[test]
    fn an_allowed_entry_with_a_relative_root_or_a_bad_account_name_is_ignored() {
        let world = World::new();
        world
            .scratch
            .write(
                "state/agent-layerz/allowed.yaml",
                "- root: relative/root\n  account: { claude: a }\n  source: s\n- root: /abs\n  account: { claude: \"Bad Name\" }\n  source: s\n",
            )
            .expect("written");
        world
            .scratch
            .set_mode("state/agent-layerz/allowed.yaml", 0o600)
            .expect("a mode");
        let run = world.which("repo");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr
                .contains("allowed.yaml: entry 1 is ignored (its root is not an absolute path)"),
            "{}",
            run.stderr
        );
        assert!(
            run.stderr.contains("allowed.yaml: entry 2 is ignored (its account is not a mapping of target to a valid account name)"),
            "{}",
            run.stderr
        );
    }

    /// Spec §13 item 19: with mise or direnv active, a user config that an `XDG_CONFIG_HOME`
    /// other than the default locates is ignored with a warning that says so, one at the
    /// default location is read, and the marker string appears in no output.
    #[test]
    fn mise_or_direnv_ignores_a_moved_user_config_and_the_marker_is_never_printed() {
        let marker = "SECRET-MARKER-4242";
        for variable in ["__MISE_DIFF", "DIRENV_DIFF"] {
            let moved = World::new();
            moved.account_folders(&["inner"]);
            moved.accounts("folders:\n  \"{root}/repo\": inner\n");
            let run = moved.run_in(
                "repo",
                &["account", "which"],
                &[(variable, marker.to_owned())],
            );
            assert!(
                run.stdout.contains(
                    "(ignored: XDG_CONFIG_HOME points away from its default and mise or direnv is active here)"
                ),
                "{variable}: {}",
                run.stdout
            );
            assert!(
                !format!("{}{}", run.stdout, run.stderr).contains(marker),
                "{variable}: the marker is printed"
            );

            let default_place = World::new();
            default_place.account_folders(&["inner"]);
            default_place
                .scratch
                .make_dirs("home/.config/agent-layerz")
                .expect("made");
            default_place
                .scratch
                .set_mode("home/.config", 0o700)
                .expect("a mode");
            default_place
                .scratch
                .write(
                    "home/.config/agent-layerz/accounts.yaml",
                    format!(
                        "folders:\n  \"{}/repo\": inner\n",
                        default_place.root.display()
                    ),
                )
                .expect("written");
            default_place
                .scratch
                .set_mode("home/.config/agent-layerz/accounts.yaml", 0o600)
                .expect("a mode");
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(default_place.root.join("repo"))
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", default_place.root.join("home"))
                .env(variable, marker)
                .timeout(Duration::from_secs(60))
                .args(["account", "which"])
                .output()
                .expect("alz runs");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("claude: inner (source: user config)"),
                "{variable}: {stdout}"
            );
            assert!(!stdout.contains(marker));
        }
    }

    /// Spec §13 item 19: a `layer.yaml` on the way to the root that cannot be used stops the
    /// walk, and a root that is not allowed gets the default account, never the outer
    /// repository's.
    #[test]
    fn an_unusable_layer_yaml_stops_the_walk_and_the_outer_repository_is_not_used() {
        let world = World::new();
        world.account_folders(&["outer"]);
        world.accounts("folders:\n  \"{root}/repo\": outer\n");
        world
            .scratch
            .write("repo/nested/layer.yaml", "stack: [unclosed\n")
            .expect("written");
        let run = world.which("repo/nested/sub");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout.contains("(layer.yaml is not valid)")
                || run.stdout.contains("(layer.yaml cannot be read)"),
            "{}",
            run.stdout
        );
        assert!(
            !run.stdout
                .contains(&format!("repository: {}/repo\n", world.root.display())),
            "the outer repository is named: {}",
            run.stdout
        );
    }

    /// Spec §13 item 19: the build output is byte-identical with and without account settings.
    #[test]
    fn the_build_is_byte_identical_with_and_without_account_settings() {
        let plain = World::new();
        let built = plain.run_in("repo", &["build"], &[]);
        assert_eq!(built.code, 0, "{}", built.stderr);
        let with = World::new();
        with.account_folders(&["inner"]);
        with.accounts("default_name: personal\nfolders:\n  \"{root}/repo\": inner\n");
        let built = with.run_in("repo", &["build"], &[]);
        assert_eq!(built.code, 0, "{}", built.stderr);
        let tree = |world: &World| {
            read_tree(&world.root.join("repo"))
                .expect("read")
                .into_iter()
                .filter(|(path, _)| !path.starts_with(".git/"))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        assert_eq!(tree(&plain), tree(&with));
    }

    /// An allowed repository whose private client layer outside the repository declares
    /// `client-a`: the build has recorded it in `layer.lock`, and `allowed.yaml` approves it.
    fn allowed_world() -> World {
        let world = World::new();
        world.account_folders(&["client-a"]);
        world.scratch.make_dirs("client").expect("made");
        world
            .scratch
            .write(
                "client/layer.yaml",
                "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
            )
            .expect("written");
        world
            .scratch
            .write(
                "repo/layer.yaml",
                "stack:\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        let built = world.run_in("repo", &["build"], &[]);
        assert_eq!(built.code, 0, "{}", built.stderr);
        world
            .scratch
            .write(
                "state/agent-layerz/allowed.yaml",
                format!(
                    "- root: \"{root}/repo\"\n  account: {{ claude: client-a, codex: client-a }}\n  source: \"{root}/client\"\n",
                    root = world.root.display()
                ),
            )
            .expect("written");
        world
            .scratch
            .set_mode("state/agent-layerz/allowed.yaml", 0o600)
            .expect("a mode");
        world
    }

    /// Spec §13 item 19: the source `layer` decides for an allowed repository, and the report
    /// shows both files.
    #[test]
    fn an_allowed_repository_gets_the_account_its_layer_names_with_the_source_layer() {
        let world = allowed_world();
        let run = world.which("repo");
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stdout
                .contains("claude: client-a (source: layer) CLAUDE_CONFIG_DIR=~/.claude-client-a"),
            "{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("(read; this repository is allowed)"),
            "{}",
            run.stdout
        );
        assert!(run.stdout.contains("accounts.yaml: ") && run.stdout.contains("(not found)"));
    }

    /// Spec §13 item 19: a cache that `XDG_CACHE_HOME` moves away from `~/.cache` while
    /// `__MISE_DIFF` exists is a cache miss, which fails closed for an allowed repository.
    #[test]
    fn a_moved_cache_under_mise_is_a_cache_miss_that_fails_closed() {
        let world = allowed_world();
        let allowed = world
            .scratch
            .read_text("state/agent-layerz/allowed.yaml")
            .expect("read");
        world
            .scratch
            .make_dirs("home/.local/state/agent-layerz")
            .expect("made");
        world
            .scratch
            .set_mode("home/.local", 0o700)
            .expect("a mode");
        world
            .scratch
            .write("home/.local/state/agent-layerz/allowed.yaml", allowed)
            .expect("written");
        world
            .scratch
            .set_mode("home/.local/state/agent-layerz/allowed.yaml", 0o600)
            .expect("a mode");
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(world.root.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", world.root.join("home"))
            .env("XDG_CACHE_HOME", world.root.join("cache"))
            .env("__MISE_DIFF", "marker-9")
            .timeout(Duration::from_secs(60))
            .args(["account", "which"])
            .output()
            .expect("alz runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{stdout}{stderr}");
        assert!(
            stdout.contains("claude: fails closed (source: layer)"),
            "{stdout}"
        );
        assert!(stdout.contains("cache miss"), "{stdout}");
        assert!(!format!("{stdout}{stderr}").contains("marker-9"));
    }

    /// Spec §13 item 19: a repository that only contains a configured account folder stays a
    /// root, and step 4 for it is a cache miss, so an allowed repository fails closed.
    #[test]
    fn a_repository_that_only_contains_a_configured_account_folder_fails_closed() {
        let world = allowed_world();
        world.scratch.make_dirs("repo/inner-account").expect("made");
        let folder = world.root.join("repo/inner-account").display().to_string();
        let run = world.run_in(
            "repo",
            &["account", "which"],
            &[("CLAUDE_CONFIG_DIR", folder)],
        );
        assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout.contains("claude: set outside alz (source: environment)")
                && run.stdout.contains("codex: fails closed (source: layer)")
                && run.stdout.contains(
                    "layer step skipped (cache miss: a layer or the repository lies inside an account folder); fails closed"
                ),
            "{}",
            run.stdout
        );

        // Not allowed: the default account, never an outer repository's.
        let plain = World::new();
        plain.scratch.make_dirs("repo/inner-account").expect("made");
        let folder = plain.root.join("repo/inner-account").display().to_string();
        let run = plain.run_in(
            "repo",
            &["account", "which"],
            &[("CLAUDE_CONFIG_DIR", folder)],
        );
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(
            run.stdout.contains("codex: default (source: default)"),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 19: `alz account which` prints no control character for a directory or
    /// source whose text holds them: a repository folder and the value of `CLAUDE_CONFIG_DIR`
    /// and `CODEX_HOME` that hold ESC, a tab, DEL, a line separator or a newline.
    #[test]
    fn account_which_prints_no_control_character_for_a_directory_or_source() {
        let world = World::new();
        let hostile = "esc\u{1b}[2Jrepo\u{7f}\u{2028}x\ty";
        for folder in [".layer", ".git"] {
            world
                .scratch
                .make_dirs(format!("{hostile}/{folder}"))
                .expect("made");
        }
        world
            .scratch
            .write(format!("{hostile}/.git/HEAD"), "ref: refs/heads/main\n")
            .expect("written");
        world
            .scratch
            .write(
                format!("{hostile}/layer.yaml"),
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        world
            .scratch
            .write(
                format!("{hostile}/.layer/layer.yaml"),
                "name: shop-api\nkind: repo\nvisibility: private\n",
            )
            .expect("written");
        let value = "/somewhere/\u{1b}]0;title\u{7}\nforged: line\u{85}";
        let run = world.run_in(
            hostile,
            &["account", "which"],
            &[
                ("CLAUDE_CONFIG_DIR", value.to_owned()),
                ("CODEX_HOME", value.to_owned()),
            ],
        );
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(run.stdout.contains("repository: "), "{}", run.stdout);
        for (stream, text) in [("stdout", &run.stdout), ("stderr", &run.stderr)] {
            assert!(
                !text
                    .chars()
                    .any(|character| character.is_control() && character != '\n'),
                "{stream}: {text:?}"
            );
            assert!(
                !text.lines().any(|line| line.starts_with("forged:")),
                "{stream}: {text:?}"
            );
        }
    }

    /// Spec §13 item 19: a case variant of the working directory resolves to the right root on
    /// a file system that folds case (the default on macOS), and the root is shown in the file
    /// system's own spelling. Where the file system keeps case (Linux), the variant is another
    /// folder: it is not the repository, and the walk never reaches the manifest of `repo`.
    #[test]
    fn a_case_variant_of_the_working_directory_resolves_to_the_real_root_or_to_nothing() {
        let world = World::new();
        if world.scratch.exists("REPO/nested/SUB") {
            let run = world.which("REPO/nested/SUB");
            assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
            assert!(
                run.stdout.starts_with(&format!(
                    "repository: {}/repo/nested\n",
                    world.root.display()
                )),
                "{}",
                run.stdout
            );
        } else {
            world.scratch.make_dirs("REPO/inner").expect("made");
            let run = world.which("REPO/inner");
            assert!(
                run.stdout.contains("no repository here"),
                "{}{}",
                run.stdout,
                run.stderr
            );
        }
    }

    /// Spec §13 item 19: a `layer.yaml` without `stack` is skipped, so the repository root is
    /// the enclosing folder that has one.
    #[test]
    fn a_layer_yaml_without_a_stack_is_skipped_on_the_way_to_the_root() {
        let world = World::new();
        world
            .scratch
            .write("repo/nested/layer.yaml", "name: not-a-stack\n")
            .expect("written");
        let run = world.which("repo/nested/sub");
        assert!(
            run.stdout
                .starts_with(&format!("repository: {}/repo\n", world.root.display())),
            "{}",
            run.stdout
        );
    }

    /// Spec §13 item 24: `alz account which` ignores a refused declaration (here the repository's
    /// own layer) with one warning, whatever the number of targets, and the client's allowed
    /// account still applies.
    #[test]
    fn a_refused_declaration_is_ignored_with_one_warning_and_the_client_account_still_applies() {
        let world = allowed_world();
        world
            .scratch
            .write(
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\naccount: client-b\n",
            )
            .expect("written");
        let run = world.which("repo");
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        let warnings = run
            .stderr
            .lines()
            .filter(|line| line.starts_with("warning [account-declaration-ignored]"))
            .count();
        assert_eq!(warnings, 1, "{}", run.stderr);
        assert!(run.stderr.contains("shop-api"), "{}", run.stderr);
        assert!(
            run.stdout.contains("claude: client-a (source: layer)")
                && run.stdout.contains("codex: client-a (source: layer)"),
            "{}",
            run.stdout
        );
    }

    /// Every `layer.yaml` that §13 item 19 lists as unusable, planted at `repo/nested/layer.yaml`
    /// (a valid stack manifest until now), with the reason `alz account which` gives for it.
    const UNUSABLE: [&str; 7] = [
        "a link",
        "a folder",
        "over 1 MiB",
        "invalid YAML",
        "past the alias limit",
        "a stack that fails the schema",
        "unreadable",
    ];

    fn plant_unusable(world: &World, kind: &str) -> &'static str {
        let manifest = "repo/nested/layer.yaml";
        world.scratch.remove_file(manifest).unwrap_or_default();
        world.scratch.remove_tree(manifest).unwrap_or_default();
        match kind {
            "a link" => {
                world
                    .scratch
                    .write(
                        "repo/real.yaml",
                        "stack:\n  - source: path:../.layer\ntargets: [claude]\n",
                    )
                    .expect("written");
                world
                    .scratch
                    .make_link(manifest, world.root.join("repo/real.yaml"))
                    .expect("linked");
                "layer.yaml cannot be read"
            }
            "a folder" => {
                world.scratch.make_dirs(manifest).expect("made");
                "layer.yaml cannot be read"
            }
            "over 1 MiB" => {
                let text = format!("stack: []\n# {}\n", "x".repeat(1_048_576));
                world.scratch.write(manifest, text).expect("written");
                "layer.yaml cannot be read"
            }
            "invalid YAML" => {
                world
                    .scratch
                    .write(manifest, "stack: [\n")
                    .expect("written");
                "layer.yaml cannot be read"
            }
            "past the alias limit" => {
                let text = format!("stack: &s []\nmore: [{}]\n", vec!["*s"; 101].join(", "));
                world.scratch.write(manifest, text).expect("written");
                "layer.yaml cannot be read"
            }
            "a stack that fails the schema" => {
                world
                    .scratch
                    .write(manifest, "stack: 5\n")
                    .expect("written");
                "layer.yaml is not valid"
            }
            "unreadable" => {
                world
                    .scratch
                    .write(
                        manifest,
                        "stack:\n  - source: path:../.layer\ntargets: [claude]\n",
                    )
                    .expect("written");
                world.scratch.set_mode(manifest, 0o000).expect("mode");
                "layer.yaml cannot be read"
            }
            other => unreachable!("an unknown kind: {other}"),
        }
    }

    /// Spec §13 item 19: a `layer.yaml` on the way to the root that cannot be read (a link, a
    /// folder, over 1 MiB, invalid YAML, past the alias limit, unreadable) or whose stack fails
    /// the schema stops the walk. An allowed root fails closed on the blocked folder, and names
    /// the file to repair; a root that is not allowed gets the default account, and neither run
    /// reads the outer repository's configuration.
    #[test]
    fn an_unusable_layer_yaml_makes_an_allowed_root_fail_closed_and_a_root_not_allowed_default() {
        if alz_test_support::running_as_root() {
            return;
        }
        for kind in UNUSABLE {
            // The outer repository is allowed with the account `outer`; the nested root must never
            // get it.
            let world = World::new();
            world.account_folders(&["outer", "client-a"]);
            let outer = format!(
                "- root: \"{root}/repo\"\n  account: {{ claude: outer, codex: outer }}\n  source: \"{root}/client\"\n",
                root = world.root.display()
            );
            let write_allowed = |text: &str| {
                world
                    .scratch
                    .write("state/agent-layerz/allowed.yaml", text)
                    .expect("written");
                world
                    .scratch
                    .set_mode("state/agent-layerz/allowed.yaml", 0o600)
                    .expect("a mode");
            };
            write_allowed(&outer);
            let reason = plant_unusable(&world, kind);

            let not_allowed = world.which("repo/nested/sub");
            let shown = format!("{}/repo/nested/layer.yaml", world.root.display());
            assert_eq!(
                not_allowed.code, 0,
                "{kind}: {}{}",
                not_allowed.stdout, not_allowed.stderr
            );
            assert!(
                not_allowed
                    .stdout
                    .contains("claude: default (source: default)"),
                "{kind}: {}",
                not_allowed.stdout
            );
            assert!(
                !not_allowed.stdout.contains("outer"),
                "{kind}: the outer repository's account applied: {}",
                not_allowed.stdout
            );
            assert!(
                not_allowed
                    .stderr
                    .contains("; the walk for the repository stops here"),
                "{kind}: {}",
                not_allowed.stderr
            );

            write_allowed(&format!(
                "{outer}- root: \"{root}/repo/nested\"\n  account: {{ claude: client-a, codex: client-a }}\n  source: \"{root}/client\"\n",
                root = world.root.display()
            ));
            let allowed = world.which("repo/nested/sub");
            assert_eq!(
                allowed.code, 1,
                "{kind}: {}{}",
                allowed.stdout, allowed.stderr
            );
            for (target, variable, folder) in [
                ("claude", "CLAUDE_CONFIG_DIR", "~/.claude-alz-blocked"),
                ("codex", "CODEX_HOME", "~/.codex-alz-blocked"),
            ] {
                assert!(
                    allowed.stdout.contains(&format!(
                        "{target}: fails closed (source: layer)\n  use step skipped (no alz use choice)\n  user config step skipped (no accounts.yaml)\n  layer step skipped ({reason}); fails closed\n  the hook applies {variable}={folder} (the blocked folder)\n  fix: repair {shown}\n"
                    )),
                    "{kind}: {}",
                    allowed.stdout
                );
            }
            assert!(
                !allowed.stdout.contains("outer"),
                "{kind}: {}",
                allowed.stdout
            );
            // Let the scratch directory be removed.
            world
                .scratch
                .set_mode("repo/nested/layer.yaml", 0o600)
                .unwrap_or_default();
        }
    }

    /// Spec §13 item 24: `alz init` prints a refused `account` declaration as a warning and
    /// still exits 0.
    #[test]
    fn init_prints_a_refused_account_declaration_as_a_warning_and_exits_zero() {
        let world = World::new();
        world
            .scratch
            .remove_file("repo/layer.yaml")
            .expect("removed");
        world.scratch.make_dirs("client").expect("made");
        world
            .scratch
            .write(
                "client/layer.yaml",
                "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
            )
            .expect("written");
        world
            .scratch
            .write(
                "repo/.layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\naccount: client-b\n",
            )
            .expect("written");
        let run = world.run_in("repo", &["init", "--stack", "../client"], &[]);
        assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
        assert!(run.stderr.contains("warning [account]"), "{}", run.stderr);
        assert!(run.stderr.contains("shop-api"), "{}", run.stderr);
    }
}
