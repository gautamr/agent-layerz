//! Spec §13 item 18, "Commit identity" (§7.12, §9.4), through the binary (TASK-55.9.9): a second
//! build changes nothing and the output stays byte-identical, a `path:` link inside the repository
//! that points outside it fails the build naming the layer, an identity that git could misread
//! fails before any git command, a gitfile `.git` and a repository root inside another work tree
//! get no write while the report still names the source of a wrong value, and a name with an
//! apostrophe gets no shell command. Fixture `26-commit-identity` (personal, client, repo); every
//! run has its own home, a global git configuration with a wrong identity, and a stand-in `git`
//! only where a test counts the calls.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_test_support::{ScratchDir, TreeFile, copy_tree, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const WRONG_NAME: &str = "Wrong Person";
    const WRONG_EMAIL: &str = "ci@example.invalid";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct Workspace {
        scratch: ScratchDir,
        work: PathBuf,
        real_path: std::ffi::OsString,
    }

    impl Workspace {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let work = scratch.join("work");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/26-commit-identity");
            copy_tree(&fixture, &work).expect("the fixture is copied");
            for folder in ["home", "bin"] {
                scratch.make_dir(folder).expect("made");
            }
            scratch
                .write(
                    "home/gitconfig",
                    format!("[user]\n\tname = {WRONG_NAME}\n\temail = {WRONG_EMAIL}\n"),
                )
                .expect("the global config is written");
            let real_path = std::ffi::OsString::from("/usr/bin:/bin");
            let workspace = Self {
                scratch,
                work,
                real_path,
            };
            workspace.git(
                &workspace.work.join("repo"),
                &["init", "--quiet", "--initial-branch=main"],
            );
            workspace
        }

        fn environment(&self, with_stand_in: bool) -> Vec<(&'static str, std::ffi::OsString)> {
            let mut path = std::ffi::OsString::new();
            if with_stand_in {
                path.push(self.scratch.join("bin"));
                path.push(":");
            }
            path.push(&self.real_path);
            vec![
                ("PATH", path),
                ("HOME", self.scratch.join("home").into_os_string()),
                (
                    "XDG_CACHE_HOME",
                    self.scratch.join("cache").into_os_string(),
                ),
                (
                    "GIT_CONFIG_GLOBAL",
                    self.scratch.join("home/gitconfig").into_os_string(),
                ),
                ("GIT_CONFIG_NOSYSTEM", "1".into()),
            ]
        }

        fn git(&self, directory: &Path, arguments: &[&str]) {
            let output = Command::new("git")
                .args(arguments)
                .current_dir(directory)
                .env_clear()
                .envs(self.environment(false))
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {arguments:?}");
        }

        fn alz_in(&self, directory: &Path, arguments: &[&str], with_stand_in: bool) -> Run {
            let mut alz: Command = cargo_bin_cmd!("alz");
            let output = alz
                .current_dir(directory)
                .env_clear()
                .envs(self.environment(with_stand_in))
                .args(arguments)
                .timeout(Duration::from_secs(60))
                .output()
                .expect("alz runs and ends in time");
            Run {
                code: output.status.code().expect("an exit code"),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        fn alz(&self, arguments: &[&str]) -> Run {
            self.alz_in(&self.work.join("repo"), arguments, false)
        }

        /// A stand-in `git` that records each call in `git-calls`.
        fn count_git_calls(&self) {
            self.scratch
                .write_executable(
                    "bin/git",
                    format!(
                        "#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n",
                        self.scratch.join("git-calls").display()
                    ),
                )
                .expect("a stand-in git");
        }

        /// The tree of the repository without the parts of `.git` that git rewrites by itself.
        fn tree_of(&self, folder: &str) -> BTreeMap<String, TreeFile> {
            read_tree(&self.work.join(folder))
                .expect("read")
                .into_iter()
                .filter(|(path, _)| !path.starts_with(".git/") || path == ".git/config")
                .collect()
        }

        fn client_declares(&self, declaration: &str) {
            self.scratch
                .write(
                    "work/client/layer.yaml",
                    format!("name: client\nkind: client\nvisibility: private\nversion: 0.1.0\ncommit_identity:\n{declaration}"),
                )
                .expect("written");
        }
    }

    /// Spec §13 item 18: `alz build --adopt` writes the declared identity into `.git/config`;
    /// a second and a third build change nothing, and the generated output stays byte-identical.
    #[test]
    fn a_second_build_changes_nothing_and_the_output_stays_byte_identical() {
        let workspace = Workspace::new();
        let first = workspace.alz(&["build", "--adopt"]);
        assert_eq!(first.code, 0, "{}", first.stderr);
        let after_first = workspace.tree_of("repo");
        let global = workspace.scratch.read("home/gitconfig").expect("read");
        for _ in 0..2 {
            let again = workspace.alz(&["build"]);
            assert_eq!(again.code, 0, "{}", again.stderr);
            assert!(!again.stdout.contains("set user."), "{}", again.stdout);
            assert_eq!(workspace.tree_of("repo"), after_first);
        }
        assert_eq!(
            workspace.scratch.read("home/gitconfig").expect("read"),
            global
        );
        assert_eq!(workspace.alz(&["check"]).code, 0);
    }

    /// Spec §13 item 18: a `path:` link inside the repository that points outside it, to a client
    /// layer that declares an identity, fails the build and names the layer.
    #[test]
    fn a_link_inside_the_repository_to_a_client_layer_fails_the_build_naming_the_layer() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .make_link("work/repo/linked-client", workspace.work.join("client"))
            .expect("a link");
        workspace
            .scratch
            .write(
                "work/repo/layer.yaml",
                "stack:\n  - source: path:../personal\n  - source: path:./linked-client\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        let run = workspace.alz(&["build"]);
        assert_eq!(run.code, 1, "{}", run.stderr);
        let line = run
            .stderr
            .lines()
            .find(|line| line.starts_with("error [identity]"))
            .unwrap_or_else(|| panic!("no identity error: {}", run.stderr));
        assert!(line.contains("client"), "{line}");
    }

    /// Spec §13 item 18: a name that has a newline, a control character, a leading `-`, a
    /// backslash, an invisible format character or an edge git would trim, and an email with a
    /// leading `-`, fails the build before any git command runs.
    #[test]
    fn an_identity_that_git_could_misread_fails_before_any_git_command() {
        let names = [
            "\"A\\nB\"",
            "\"A\\x01B\"",
            "\"-flag\"",
            "\"A\\\\B\"",
            "\"A\\u200bB\"",
            "\"Owner.\"",
            "\"Owner,\"",
            "\";Owner\"",
        ];
        let declarations = names
            .iter()
            .map(|name| format!("  name: {name}\n  email: owner@client.example\n"))
            .chain(std::iter::once(
                "  name: Owner Example\n  email: \"-x@client.example\"\n".to_owned(),
            ));
        for declaration in declarations {
            let workspace = Workspace::new();
            workspace.client_declares(&declaration);
            workspace.count_git_calls();
            let run = workspace.alz_in(&workspace.work.join("repo"), &["build"], true);
            assert_eq!(run.code, 1, "{declaration}: {}", run.stderr);
            assert!(
                run.stderr.contains("error ["),
                "{declaration}: {}",
                run.stderr
            );
            assert!(
                !workspace.scratch.exists("git-calls"),
                "{declaration}: git ran"
            );
        }
    }

    /// Spec §13 item 18: a gitfile `.git` gets no write, the report still names the source of
    /// the wrong value, and never the value.
    #[test]
    fn a_gitfile_dot_git_gets_no_write_and_the_report_names_the_source() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .rename("work/repo/.git", "work/git-elsewhere")
            .expect("moved");
        workspace
            .scratch
            .write(
                "work/repo/.git",
                format!(
                    "gitdir: {}\n",
                    workspace.work.join("git-elsewhere").display()
                ),
            )
            .expect("a gitfile");
        let config = workspace
            .scratch
            .read("work/git-elsewhere/config")
            .expect("read");
        let run = workspace.alz(&["build", "--adopt"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            workspace
                .scratch
                .read("work/git-elsewhere/config")
                .expect("read"),
            config
        );
        assert!(
            run.stderr.contains("from global config (~/gitconfig)"),
            "{}",
            run.stderr
        );
        assert!(!format!("{}{}", run.stdout, run.stderr).contains(WRONG_EMAIL));
        assert!(!run.stdout.contains("set user."), "{}", run.stdout);
    }

    /// Spec §13 item 18: a repository root inside another work tree gets no write: the outer
    /// work tree's configuration is unchanged, no `.git` is made, and the source is named.
    #[test]
    fn a_repository_root_inside_another_work_tree_gets_no_write() {
        let workspace = Workspace::new();
        workspace.scratch.make_dirs("work/outer").expect("made");
        workspace.git(
            &workspace.work.join("outer"),
            &["init", "--quiet", "--initial-branch=main"],
        );
        workspace
            .scratch
            .rename("work/repo", "work/outer/inner")
            .expect("moved");
        for layer in ["personal", "client"] {
            workspace
                .scratch
                .rename(format!("work/{layer}"), format!("work/outer/{layer}"))
                .expect("moved");
        }
        workspace
            .scratch
            .remove_tree("work/outer/inner/.git")
            .expect("removed");
        let outer_config = workspace
            .scratch
            .read("work/outer/.git/config")
            .expect("read");
        let inner = workspace.work.join("outer/inner");
        let run = workspace.alz_in(&inner, &["build", "--adopt"], false);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert_eq!(
            workspace
                .scratch
                .read("work/outer/.git/config")
                .expect("read"),
            outer_config
        );
        assert!(!workspace.scratch.exists("work/outer/inner/.git"));
        assert!(!run.stdout.contains("set user."), "{}", run.stdout);
        assert!(
            run.stderr.contains("from global config (~/gitconfig)"),
            "{}",
            run.stderr
        );
    }

    /// Spec §13 item 18: a name with an apostrophe gets no shell command in the report.
    #[test]
    fn a_name_with_an_apostrophe_gets_no_shell_command() {
        let workspace = Workspace::new();
        workspace.client_declares("  name: \"O'Brien\"\n  email: owner@client.example\n");
        let run = workspace.alz(&["build"]);
        assert_eq!(run.code, 0, "{}", run.stderr);
        assert!(
            run.stderr.contains("[identity]"),
            "the wrong identity is reported: {}",
            run.stderr
        );
        assert!(
            !run.stderr
                .lines()
                .any(|line| line.contains("git config") && line.contains("Brien")),
            "{}",
            run.stderr
        );
    }

    /// Spec §13 item 18: a declaration in the repo layer (even with `kind: client`), in a group
    /// layer, in a public client layer, or a second client layer fails the build and names the
    /// layer.
    #[test]
    fn a_layer_that_may_not_declare_an_identity_fails_the_build_and_names_it() {
        let declaration =
            "commit_identity:\n  name: Owner Example\n  email: owner@client.example\n";
        let cases = [
            ("the repo layer", "repo"),
            ("a group layer", "group"),
            ("a public client layer", "public-client"),
            ("a second client layer", "client-b"),
        ];
        for (what, layer) in cases {
            let workspace = Workspace::new();
            let name = layer;
            let (file, stack) = match layer {
                "repo" => (
                    "work/repo/.layer/layer.yaml".to_owned(),
                    "  - source: path:../personal\n  - source: path:../client\n  - source: path:./.layer\n".to_owned(),
                ),
                "public-client" => (
                    "work/public-client/layer.yaml".to_owned(),
                    "  - source: path:../personal\n  - source: path:../public-client\n  - source: path:./.layer\n".to_owned(),
                ),
                other => (
                    format!("work/{other}/layer.yaml"),
                    format!("  - source: path:../personal\n  - source: path:../client\n  - source: path:../{other}\n  - source: path:./.layer\n"),
                ),
            };
            let (kind, visibility) = match layer {
                "group" => ("group", "private"),
                "public-client" => ("client", "public"),
                "client-b" => ("client", "private"),
                _ => ("repo", "private"),
            };
            workspace
                .scratch
                .make_dirs(format!("work/{name}"))
                .expect("made");
            workspace
                .scratch
                .write(
                    &file,
                    format!("name: {name}\nkind: {kind}\nvisibility: {visibility}\nversion: 0.1.0\n{declaration}"),
                )
                .expect("written");
            workspace
                .scratch
                .write(
                    "work/repo/layer.yaml",
                    format!("stack:\n{stack}targets: [claude]\n"),
                )
                .expect("written");
            let run = workspace.alz(&["build"]);
            assert_eq!(run.code, 1, "{what}: {}", run.stderr);
            let line = run
                .stderr
                .lines()
                .find(|line| line.starts_with("error [identity]"))
                .unwrap_or_else(|| panic!("{what}: no identity error: {}", run.stderr));
            assert!(line.contains(name), "{what}: {line}");
        }
    }

    /// Spec §13 item 18: the Commits section names the layer that declares the identity, and
    /// without a declaration `AGENTS.md` has no Commits section.
    #[test]
    fn the_commits_section_names_the_declaring_layer_and_is_absent_without_a_declaration() {
        let workspace = Workspace::new();
        assert_eq!(workspace.alz(&["build"]).code, 0);
        let with = workspace
            .scratch
            .read_text("work/repo/AGENTS.md")
            .expect("read");
        let section = with
            .lines()
            .skip_while(|line| !line.contains("Commits"))
            .take(6)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(section.contains("client"), "{with}");
        let without = Workspace::new();
        for layer in ["personal", "client"] {
            without
                .scratch
                .write(
                    format!("work/{layer}/layer.yaml"),
                    format!(
                        "name: {layer}\nkind: {}\nvisibility: {}\nversion: 0.1.0\n",
                        if layer == "personal" {
                            "personal"
                        } else {
                            "client"
                        },
                        if layer == "personal" {
                            "public"
                        } else {
                            "private"
                        }
                    ),
                )
                .expect("written");
        }
        assert_eq!(without.alz(&["build"]).code, 0);
        let text = without
            .scratch
            .read_text("work/repo/AGENTS.md")
            .expect("read");
        assert!(!text.contains("Commits"), "{text}");
    }

    /// Spec §13 item 18: a `git:` layer at a local path inside the repository, or reached by a
    /// `file://` URL of that path, counts as inside it, so its identity declaration fails the
    /// build and names the layer.
    #[test]
    fn a_git_layer_inside_the_repository_counts_as_inside_by_path_and_by_file_url() {
        for source in [
            "git:./vendor/client@main",
            "git:file://{repo}/vendor/client@main",
        ] {
            let workspace = Workspace::new();
            workspace
                .scratch
                .rename("work/client", "work/repo/vendor-client")
                .expect("moved");
            workspace
                .scratch
                .make_dirs("work/repo/vendor")
                .expect("made");
            workspace
                .scratch
                .rename("work/repo/vendor-client", "work/repo/vendor/client")
                .expect("moved");
            let client = workspace.work.join("repo/vendor/client");
            workspace.git(&client, &["init", "--quiet", "--initial-branch=main"]);
            workspace.git(&client, &["add", "-A"]);
            workspace.git(
                &client,
                &[
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.com",
                    "commit",
                    "--quiet",
                    "-m",
                    "layer",
                ],
            );
            let real = workspace.scratch.real_path().expect("a real path");
            let source = source.replace("{repo}", &real.join("work/repo").display().to_string());
            workspace
                .scratch
                .write(
                    "work/repo/layer.yaml",
                    format!("stack:\n  - source: path:../personal\n  - source: {source}\n  - source: path:./.layer\ntargets: [claude]\n"),
                )
                .expect("written");
            let run = workspace.alz(&["build"]);
            assert_eq!(run.code, 1, "{source}: {}", run.stderr);
            assert!(
                run.stderr.contains("error [identity]") && run.stderr.contains("client"),
                "{source}: {}",
                run.stderr
            );
        }
    }

    /// Spec §13 item 18: a link outside the repository that points into it, and a case variant
    /// of an in-repo path, count as inside it; the layer's identity declaration fails the
    /// build. On a case-sensitive file system the case variant names no folder, and the build
    /// fails for that reason instead.
    #[test]
    fn a_link_into_the_repository_and_a_case_variant_of_an_in_repo_path_count_as_inside() {
        let workspace = Workspace::new();
        workspace
            .scratch
            .make_dirs("work/repo/vendor")
            .expect("made");
        workspace
            .scratch
            .rename("work/client", "work/repo/vendor/client")
            .expect("moved");
        workspace
            .scratch
            .make_link(
                "work/link-to-client",
                workspace.work.join("repo/vendor/client"),
            )
            .expect("a link");
        let manifest = |source: &str| {
            workspace
                .scratch
                .write(
                    "work/repo/layer.yaml",
                    format!("stack:\n  - source: path:../personal\n  - source: {source}\n  - source: path:./.layer\ntargets: [claude]\n"),
                )
                .expect("written");
        };
        manifest("path:../link-to-client");
        let linked = workspace.alz(&["build"]);
        assert_eq!(linked.code, 1, "{}", linked.stderr);
        assert!(
            linked.stderr.contains("error [identity]"),
            "{}",
            linked.stderr
        );
        manifest("path:./VENDOR/Client");
        let variant = workspace.alz(&["build"]);
        assert_eq!(variant.code, 1, "{}", variant.stderr);
        let case_insensitive = workspace.scratch.exists("work/repo/VENDOR");
        let expected = if case_insensitive {
            "error [identity]"
        } else {
            "error [source]"
        };
        assert!(variant.stderr.contains(expected), "{}", variant.stderr);
    }
}
