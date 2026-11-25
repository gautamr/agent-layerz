//! `alz lint` in a repository (spec §6, §10, §10.1): every stack entry is resolved by its source's
//! rule, the stack is validated, and the summary names the layers (TASK-55.4.10). Each test builds
//! its own machine under one real path: a home folder, a cache base, the repository that is the
//! working directory, and real git repositories as the sources. `HOME`, `XDG_CACHE_HOME` and
//! `PATH` are set for each run, and nothing else of the test's environment reaches `alz`.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_core::layer::{LayerName, Visibility};
    use alz_core::lock::{ContentHash, LayerFiles, Lock, LockedLayer, ObjectId};
    use alz_core::path::SafeRelPath;
    use alz_core::stack::Source;
    use alz_io::EnvSnapshot;
    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const LAYER: &str = "name: team\nkind: client\nvisibility: private\n";
    const CREDENTIALS_MESSAGE: &str = "error [source] a git: source in layer.yaml holds credentials in its URL; remove them and use a git credential helper\n";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    struct World {
        scratch: ScratchDir,
        root: PathBuf,
    }

    fn real_path() -> std::ffi::OsString {
        EnvSnapshot::capture()
            .vars()
            .get("PATH")
            .expect("PATH is set")
            .to_owned()
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            for name in ["home", "repo", "bin"] {
                scratch.make_dir(name).expect("made");
            }
            Self { scratch, root }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn put(&self, relative: &str, text: &str) {
            if let Some(parent) = Path::new(relative).parent() {
                self.scratch.make_dirs(parent).expect("made");
            }
            self.scratch.write(relative, text).expect("written");
        }

        fn manifest(&self, text: &str) {
            self.put("repo/layer.yaml", text);
        }

        fn lock(&self, layers: Vec<LockedLayer>) {
            let lock = Lock {
                visibility: Visibility::Private,
                layers,
                files: std::collections::BTreeMap::new(),
            };
            self.put("repo/layer.lock", &lock.emit());
        }

        fn cache_names(&self) -> Vec<String> {
            self.scratch.list("xdg/agent-layerz")
        }

        fn alz(&self) -> Run {
            self.alz_with_path(&self.path_with_bin())
        }

        /// `alz lint` with a `PATH` that holds no git at all: only the empty `bin` folder.
        fn alz_without_git(&self) -> Run {
            assert!(!self.scratch.exists("bin/git"), "bin holds a git");
            self.alz_with_path(self.path("bin").as_os_str())
        }

        fn alz_with_path(&self, path: &std::ffi::OsStr) -> Run {
            let mut command: Command = cargo_bin_cmd!("alz");
            let output = command
                .current_dir(self.path("repo"))
                .env_clear()
                .env("PATH", path)
                .env("HOME", self.path("home"))
                .env("XDG_CACHE_HOME", self.path("xdg"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", self.path("gitconfig"))
                .env("ALZ_FAKE_REMOTE", self.path("work"))
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

        fn path_with_bin(&self) -> std::ffi::OsString {
            let mut path = self.path("bin").into_os_string();
            path.push(":");
            path.push(real_path());
            path
        }

        fn git(&self, directory: &Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .current_dir(directory)
                .env_clear()
                .env("PATH", real_path())
                .env("HOME", self.path("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args([
                    "-c",
                    "init.defaultBranch=main",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {args:?}: {output:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        }

        /// Commits `files` in the repository `relative` (made when missing) and gives the commit.
        fn commit(&self, relative: &str, files: &[(&str, &str)]) -> ObjectId {
            let directory = self.path(relative);
            if !self.scratch.exists(format!("{relative}/.git")) {
                self.scratch.make_dirs(relative).expect("made");
                self.git(&directory, &["init", "-q"]);
            }
            for (name, text) in files {
                self.put(&format!("{relative}/{name}"), text);
            }
            self.git(&directory, &["add", "-A"]);
            self.git(&directory, &["commit", "-q", "-m", "commit"]);
            ObjectId::try_from(self.git(&directory, &["rev-parse", "HEAD"]).as_str())
                .expect("an id")
        }
    }

    fn rel(text: &str) -> SafeRelPath {
        SafeRelPath::try_from(text).expect("a path")
    }

    fn locked(source: &str, sha: Option<ObjectId>, files: &[(&str, &str)]) -> LockedLayer {
        LockedLayer {
            name: LayerName::try_from("team").expect("a name"),
            source: Source::try_from(source).expect("a source"),
            rev: sha.map(|sha| sha.short_rev()),
            sha,
            content_hash: ContentHash::of(&LayerFiles::from_contents(
                files
                    .iter()
                    .map(|(path, text)| (rel(path), text.as_bytes())),
            )),
        }
    }

    // --- AC #1: every entry is resolved, the stack validated, the summary printed ---------------

    #[test]
    fn a_stack_of_path_layers_resolves_and_prints_the_summary_without_touching_the_cache() {
        let world = World::new();
        world.put(
            "personal/layer.yaml",
            "name: personal\nkind: personal\nvisibility: public\n",
        );
        world.put(
            "repo/.layer/layer.yaml",
            "name: repo\nkind: repo\nvisibility: private\n",
        );
        world.manifest("stack:\n  - source: path:../personal\n  - source: path:./.layer\n");
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: personal, repo\n");
        assert!(
            !world.scratch.exists("xdg"),
            "a stack of working trees makes no cache"
        );
        assert!(!run.stdout.contains("not-ported") && !run.stderr.contains("not-ported"));
    }

    #[test]
    fn the_leak_guard_checks_each_public_layer_of_a_stack_even_when_another_entry_failed() {
        let world = World::new();
        world.put(
            "personal/layer.yaml",
            "name: personal\nkind: personal\nvisibility: public\nforbid_patterns:\n  - hidden-client\n",
        );
        world.put("personal/notes.md", "for hidden-client\n");
        world.put(
            "repo/.layer/layer.yaml",
            "name: repo\nkind: repo\nvisibility: private\n",
        );
        world.put("repo/.layer/notes.md", "for hidden-client\n");
        world.manifest(
            "stack:\n  - source: path:../personal\n  - source: path:../gone\n  - source: path:./.layer\n",
        );
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains(
                "error [leak-forbid] personal:notes.md: matches forbid_patterns entry #1\n"
            ),
            "{}",
            run.stderr
        );
        assert!(!run.stderr.contains("repo:notes.md"), "{}", run.stderr);
        assert!(!run.stderr.contains("hidden-client"), "{}", run.stderr);
    }

    #[test]
    fn several_errors_of_a_stack_are_reported_together_in_stack_order() {
        let world = World::new();
        world.put("bad/layer.yaml", "name: bad\nkind: personal\n");
        world.put(
            "envy/layer.yaml",
            "name: envy\nkind: personal\nvisibility: private\n",
        );
        world.put("envy/.env.dev", "SECRET=1\n");
        world.put(
            "pub/layer.yaml",
            "name: pub\nkind: personal\nvisibility: public\n",
        );
        world.manifest(
            "stack:\n  - source: path:../gone\n  - source: path:../bad\n  - source: path:../envy\n  - source: path:../pub\n",
        );
        let run = world.alz();
        assert_eq!(run.code, 1);
        let lines: Vec<&str> = run.stderr.lines().collect();
        assert_eq!(lines.len(), 4, "stderr: {}", run.stderr);
        assert_eq!(
            lines[0],
            "error [source] layer directory not found for 'path:../gone'"
        );
        assert!(
            lines[1].starts_with("error [layer] path:../bad:layer.yaml:"),
            "{}",
            lines[1]
        );
        assert_eq!(
            lines[2],
            "error [env-file] envy:.env.dev: layers must not contain .env files; remove it"
        );
        assert_eq!(
            lines[3],
            "error [visibility] path:../pub: public layer 'pub' cannot extend private layer 'envy'; order the stack from most public to most private"
        );
        assert_eq!(
            run.stdout,
            "lint failed: 4 errors in path:../gone, path:../bad, envy, pub\n"
        );
        assert!(!run.stderr.contains("SECRET"));
    }

    #[test]
    fn two_layers_with_one_name_are_the_error_duplicate_layer() {
        let world = World::new();
        world.put(
            "a/layer.yaml",
            "name: same\nkind: personal\nvisibility: public\n",
        );
        world.put(
            "b/layer.yaml",
            "name: same\nkind: personal\nvisibility: public\n",
        );
        world.manifest("stack:\n  - source: path:../a\n  - source: path:../b\n");
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [duplicate-layer] layer name 'same' is used by both 'path:../a' and 'path:../b'\n"
        );
        assert_eq!(run.stdout, "lint failed: 1 error in same, same\n");
    }

    #[test]
    fn a_layer_yaml_that_is_a_link_or_missing_is_the_error_layer() {
        let world = World::new();
        world.put("linked/real.yaml", LAYER);
        world
            .scratch
            .make_link("linked/layer.yaml", "real.yaml")
            .expect("linked");
        world.scratch.make_dir("empty").expect("made");
        world.manifest("stack:\n  - source: path:../linked\n  - source: path:../empty\n");
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr,
            "error [layer] layer.yaml in layer 'path:../linked' must be a regular file, not a link or a folder\nerror [layer] missing layer.yaml in layer 'path:../empty'\n"
        );
    }

    #[test]
    fn an_invalid_manifest_stops_before_any_layer_is_resolved() {
        let world = World::new();
        world.manifest("stack:\n  - source: path:../a\n    ref: [x]\n");
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert!(run.stderr.starts_with("error [manifest]"), "{}", run.stderr);
        assert_eq!(run.stdout, "");
    }

    // --- git: and github: layers ----------------------------------------------------------------

    #[test]
    fn a_git_layer_from_a_file_url_is_cloned_into_the_cache_and_linted() {
        let world = World::new();
        world.commit("work", &[("layer.yaml", LAYER)]);
        world.manifest(&format!(
            "stack:\n  - source: git:file://{}@main\n",
            world.path("work").display()
        ));
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
        let names = world.cache_names();
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("git-"), "{names:?}");
    }

    #[test]
    fn a_git_layer_at_a_local_path_is_cloned_and_linted() {
        let world = World::new();
        world.commit("work", &[("layer.yaml", LAYER)]);
        world.manifest(&format!(
            "stack:\n  - source: git:{}@main\n",
            world.path("work").display()
        ));
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
    }

    #[test]
    fn a_github_layer_is_cloned_through_the_users_git_configuration() {
        let world = World::new();
        world.commit("work", &[("layer.yaml", LAYER)]);
        // The mirror: the user's `insteadOf` sends the https URL to an ssh remote, which the fake
        // `ssh` answers from the local repository. No network is used.
        world.put(
            "gitconfig",
            "[url \"ssh://mirror/work\"]\n\tinsteadOf = https://github.com/o/r.git\n",
        );
        world
            .scratch
            .write_executable(
                "bin/ssh",
                "#!/bin/sh\nexec git upload-pack \"$ALZ_FAKE_REMOTE\"\n",
            )
            .expect("written");
        world.manifest("stack:\n  - source: github:o/r@main\n");
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
        assert_eq!(world.cache_names().len(), 1);
    }

    #[test]
    fn a_failed_fetch_is_the_error_source_with_the_source_and_git_s_line() {
        let world = World::new();
        let source = format!("git:file://{}@main", world.path("missing").display());
        world.manifest(&format!("stack:\n  - source: {source}\n"));
        let run = world.alz();
        assert_eq!(run.code, 1);
        let prefix = format!("error [source] cannot fetch '{source}'");
        assert!(run.stderr.starts_with(&prefix), "{}", run.stderr);
        assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
        assert_eq!(run.stdout, format!("lint failed: 1 error in {source}\n"));
    }

    // --- AC #3: the lock checks a cached layer, never a working tree ----------------------------

    #[test]
    fn an_unpinned_path_layer_is_read_in_place_and_never_checked_against_the_lock() {
        let world = World::new();
        world.put(
            "personal/layer.yaml",
            "name: team\nkind: personal\nvisibility: public\n",
        );
        world.manifest("stack:\n  - source: path:../personal\n");
        world.lock(vec![locked(
            "path:../personal",
            None,
            &[("layer.yaml", "other\n")],
        )]);
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
        // An edit after the lock is not a mismatch either.
        world.put("personal/notes.md", "edited\n");
        assert_eq!(world.alz().code, 0);
    }

    #[test]
    fn a_cached_git_layer_is_checked_against_the_lock_and_read_without_git_when_it_matches() {
        let world = World::new();
        let sha = world.commit("work", &[("layer.yaml", LAYER)]);
        // A bare copy, so that the source's clone argument is the same after it is removed.
        world.git(
            &world.root,
            &[
                "clone",
                "-q",
                "--bare",
                world.path("work").to_str().unwrap(),
                world.path("bare.git").to_str().unwrap(),
            ],
        );
        let source = format!("git:file://{}@main", world.path("bare.git").display());
        world.manifest(&format!("stack:\n  - source: {source}\n"));
        assert_eq!(world.alz().code, 0);

        world.lock(vec![locked(&source, Some(sha), &[("layer.yaml", LAYER)])]);
        // With the source repository gone, only the cache can answer.
        world.scratch.remove_tree("bare.git").expect("removed");
        // And with no git on PATH: a layer that the lock pins and the cache holds starts no git
        // and asks for no version.
        let run = world.alz_without_git();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");

        world.lock(vec![locked(
            &source,
            Some(sha),
            &[("layer.yaml", "other\n")],
        )]);
        let run = world.alz_without_git();
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [cache] cached layer 'team'")
                && run.stderr.contains("no longer matches layer.lock"),
            "{}",
            run.stderr
        );
    }

    /// Spec §6 "Validating the stack": the refusals of a layer's files are the same for a `git:`
    /// clone as for a working tree, with the cache's error for the second. A name that is not valid
    /// UTF-8 is the error `layer` (the clone case with such a name is in `git_sources`, on Linux); a
    /// name that is not a safe path is the error `cache` `cannot store '<source>' in the layer
    /// cache at <dir>`, the name is never printed, and nothing is published to the cache.
    #[test]
    fn a_clone_with_a_name_that_is_not_a_safe_path_is_the_cache_error_cannot_store() {
        let world = World::new();
        world.commit("work", &[("layer.yaml", LAYER), ("a\\b.md", "text\n")]);
        let source = format!("git:file://{}@main", world.path("work").display());
        world.manifest(&format!("stack:\n  - source: {source}\n"));
        let run = world.alz();
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(
            run.stderr,
            format!(
                "error [cache] cannot store '{source}' in the layer cache at {}\n",
                world.path("xdg/agent-layerz").display()
            )
        );
        assert!(!run.stderr.contains("a\\b"), "{}", run.stderr);
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with("git-")),
            "{:?}",
            world.cache_names()
        );
    }

    #[test]
    fn a_pinned_path_layer_is_read_from_the_cache_and_checked_against_the_lock() {
        let world = World::new();
        let sha = world.commit("hub", &[("layer/layer.yaml", LAYER)]);
        world.git(&world.path("hub"), &["tag", "v1"]);
        world.manifest("stack:\n  - source: path:../hub/layer\n    ref: v1\n");
        let run = world.alz();
        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
        assert!(
            world
                .cache_names()
                .iter()
                .any(|name| name.starts_with("tree2@")),
            "{:?}",
            world.cache_names()
        );
        // The working tree moves on; the layer is the one at the tag.
        world.put(
            "hub/layer/layer.yaml",
            "name: other\nkind: client\nvisibility: private\n",
        );
        assert_eq!(world.alz().stdout, "lint ok: team\n");

        world.lock(vec![locked(
            "path:../hub/layer",
            Some(sha),
            &[("layer.yaml", "other\n")],
        )]);
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [cache] cached layer 'team'")
                && run.stderr.contains("no longer matches layer.lock"),
            "{}",
            run.stderr
        );
    }

    // --- AC #1: what the cache holds, and what it must never trust ------------------------------

    #[test]
    fn two_git_repositories_whose_flattened_paths_are_equal_are_kept_in_separate_entries() {
        let world = World::new();
        // `acme-corp/layer` and `acme/corp/layer` flatten to the same name.
        let sha_a = world.commit(
            "remotes/acme-corp/layer",
            &[(
                "layer.yaml",
                "name: alpha\nkind: client\nvisibility: private\n",
            )],
        );
        let sha_b = world.commit(
            "remotes/acme/corp/layer",
            &[(
                "layer.yaml",
                "name: beta\nkind: client\nvisibility: private\n",
            )],
        );
        world.manifest(&format!(
            "stack:\n  - source: git:{}@main\n",
            world.path("remotes/acme-corp/layer").display()
        ));
        let first = world.alz();
        assert_eq!((first.code, first.stdout.as_str()), (0, "lint ok: alpha\n"));
        world.manifest(&format!(
            "stack:\n  - source: git:{}@main\n",
            world.path("remotes/acme/corp/layer").display()
        ));
        let second = world.alz();
        assert_eq!(
            (second.code, second.stdout.as_str()),
            (0, "lint ok: beta\n")
        );

        let names = world.cache_names();
        let split: Vec<(&str, &str)> = names
            .iter()
            .map(|name| name.split_once('@').expect("a key and a commit"))
            .collect();
        assert_eq!(split.len(), 2, "{names:?}");
        assert!(
            split.iter().all(|(key, _)| key.starts_with("git-")),
            "{names:?}"
        );
        assert_ne!(
            split[0].0, split[1].0,
            "one key for two repositories: {names:?}"
        );
        let mut commits: Vec<String> = split.iter().map(|(_, sha)| (*sha).to_owned()).collect();
        commits.sort();
        let mut expected = vec![sha_a.to_string(), sha_b.to_string()];
        expected.sort();
        assert_eq!(commits, expected);
    }

    #[test]
    fn an_entry_of_the_older_tree_format_is_ignored_and_the_layer_is_written_again() {
        let world = World::new();
        world.commit("work", &[("layer.yaml", LAYER)]);
        let tree = world.git(&world.path("work"), &["rev-parse", "HEAD^{tree}"]);
        // A stale entry of the old format, with content that differs from the tree.
        let stale = format!("xdg/agent-layerz/tree@{tree}/layer.yaml");
        world.put(&stale, "name: stale\n");
        world.manifest("stack:\n  - source: path:../work\n    ref: main\n");

        let run = world.alz();

        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
        assert_eq!(run.stdout, "lint ok: team\n");
        let current = format!("xdg/agent-layerz/tree2@{tree}/layer.yaml");
        assert_eq!(world.scratch.read_text(&current).expect("written"), LAYER);
        assert_eq!(
            world.scratch.read_text(&stale).expect("left alone"),
            "name: stale\n"
        );
    }

    /// A repository whose `git:` layer is cached and locked; gives the entry's path under the root.
    fn cached_and_locked_git_layer(world: &World) -> String {
        let sha = world.commit("work", &[("layer.yaml", LAYER)]);
        let source = format!("git:file://{}@main", world.path("work").display());
        world.manifest(&format!("stack:\n  - source: {source}\n"));
        assert_eq!(world.alz().code, 0);
        world.lock(vec![locked(&source, Some(sha), &[("layer.yaml", LAYER)])]);
        let names = world.cache_names();
        assert_eq!(names.len(), 1, "{names:?}");
        assert_eq!(world.alz().code, 0);
        format!("xdg/agent-layerz/{}", names[0])
    }

    #[test]
    fn a_cached_layer_yaml_that_was_tampered_with_is_the_cache_error_and_is_never_quoted() {
        let world = World::new();
        let entry = cached_and_locked_git_layer(&world);
        world.put(&format!("{entry}/layer.yaml"), "name: [SECRET-MARKER\n");

        let run = world.alz();

        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [cache] cached layer 'team'"),
            "{}",
            run.stderr
        );
        assert!(
            !format!("{}{}", run.stdout, run.stderr).contains("SECRET-MARKER"),
            "{}{}",
            run.stdout,
            run.stderr
        );
    }

    #[test]
    fn a_cached_entry_without_its_layer_yaml_is_the_cache_error() {
        let world = World::new();
        let entry = cached_and_locked_git_layer(&world);
        world
            .scratch
            .remove_file(format!("{entry}/layer.yaml"))
            .expect("removed");

        let run = world.alz();

        assert_eq!(run.code, 1);
        assert!(
            run.stderr.starts_with("error [cache] cached layer 'team'")
                && run.stderr.contains("delete that directory"),
            "{}",
            run.stderr
        );
    }

    // --- AC #2 and the refusals before any access ------------------------------------------------

    /// A `git` in front of the real one that records that it was run.
    fn plant_git_spy(world: &World) {
        world
            .scratch
            .write_executable(
                "bin/git",
                format!(
                    "#!/bin/sh\ntouch '{}'\nexit 1\n",
                    world.path("git-ran").display()
                ),
            )
            .expect("written");
    }

    #[test]
    fn a_source_with_credentials_is_refused_before_git_or_the_cache_and_the_token_is_never_printed()
    {
        for source in [
            "git:https://user:token-7f3a@host.example/x.git@v1",
            "git:https://token-7f3a@host.example/x.git@v1",
            "git:ssh://git:token-7f3a@host.example/x.git@v1",
            "git:https://token-7f3a@host.example/x.git",
            "git:me:token-7f3a@host.example/o/r",
            "git:user:token-7f3a@host.example:repo@v1",
        ] {
            let world = World::new();
            plant_git_spy(&world);
            world.manifest(&format!("stack:\n  - source: {source}\n"));
            let run = world.alz();
            assert_eq!(run.code, 1, "{source}");
            assert_eq!(run.stderr, CREDENTIALS_MESSAGE, "{source}");
            assert!(!run.stdout.contains("token-7f3a") && !run.stderr.contains("token-7f3a"));
            assert!(!world.scratch.exists("git-ran"), "git ran for {source}");
            assert!(
                !world.scratch.exists("xdg"),
                "a cache was made for {source}"
            );
        }
    }

    #[test]
    fn a_user_name_without_a_password_is_not_refused_as_credentials() {
        for source in [
            "git:ssh://git@host.example/x.git@v1",
            "git:git@host.example:o/x.git@v1",
            "git:git@host.example:repo.git@v1",
            "git:host.invalid:client.git@v1",
            "git:x:y@v1",
        ] {
            let world = World::new();
            plant_git_spy(&world);
            world.manifest(&format!("stack:\n  - source: {source}\n"));
            let run = world.alz();
            assert!(
                !run.stderr.contains("credentials"),
                "{source}: {}",
                run.stderr
            );
        }
    }

    #[test]
    fn a_transport_helper_url_is_a_manifest_error_and_git_never_runs() {
        let world = World::new();
        plant_git_spy(&world);
        world.manifest(
            "stack:\n  - source: \"git:ext::sh -c touch% /x@v1\"\n  - source: \"git:https::https://tok@host/x.git@v1\"\n",
        );
        let run = world.alz();
        assert_eq!(run.code, 1);
        assert_eq!(
            run.stderr.matches("transport helper").count(),
            2,
            "{}",
            run.stderr
        );
        assert!(!run.stderr.contains("tok@host"), "{}", run.stderr);
        assert!(!world.scratch.exists("git-ran"));
        assert!(!world.scratch.exists("xdg"));
    }

    #[test]
    fn a_layer_in_an_account_folder_or_at_home_is_refused_before_anything_is_read() {
        let world = World::new();
        plant_git_spy(&world);
        world.put("home/.claude-work/layer.yaml", LAYER);
        world.put("repo/.layer/layer.yaml", LAYER);
        world
            .scratch
            .make_link("repo/via", "../home/.claude-work")
            .expect("linked");
        world.manifest(&format!(
            "stack:\n  - source: path:../home/.claude-work\n  - source: path:./via\n  - source: path:../home\n  - source: git:{}/home/.claude-work@main\n",
            world.root.display()
        ));
        let run = world.alz();
        assert_eq!(run.code, 1);
        let lines: Vec<&str> = run.stderr.lines().collect();
        assert_eq!(lines.len(), 4, "{}", run.stderr);
        for line in &lines[..3] {
            assert!(
                line.starts_with("error [source] layer 'path:")
                    && line.ends_with(
                        "' touches an account folder or the home folder; alz never reads there"
                    ),
                "{line}"
            );
        }
        assert!(
            lines[3].starts_with("error [source] layer 'git:")
                && lines[3].ends_with(
                    "touches an account folder or the home folder; alz never reads there"
                ),
            "{}",
            lines[3]
        );
        assert!(!world.scratch.exists("git-ran"));
        assert!(!world.scratch.exists("xdg"));
    }

    #[test]
    fn a_layer_in_the_target_of_a_link_named_like_an_account_folder_is_refused_before_anything_is_read()
     {
        let world = World::new();
        world.put("home/dotfiles/claude/layer.yaml", LAYER);
        world.put("repo/.layer/layer.yaml", LAYER);
        world
            .scratch
            .make_link("home/.claude-x", "dotfiles/claude")
            .expect("linked");
        world
            .scratch
            .make_link("repo/via", "../home/dotfiles/claude")
            .expect("linked");
        world.manifest(
            "stack:\n  - source: path:../home/dotfiles/claude\n  - source: path:./via\n  - source: path:./.layer\n",
        );
        // Sealed: a read of anything in the folder fails loudly instead of being refused.
        world
            .scratch
            .set_mode("home/dotfiles/claude", 0o000)
            .expect("sealed");
        let run = world.alz();
        world
            .scratch
            .set_mode("home/dotfiles/claude", 0o700)
            .expect("unsealed");
        assert_eq!(run.code, 1, "{}", run.stderr);
        let lines: Vec<&str> = run.stderr.lines().collect();
        assert_eq!(lines.len(), 2, "{}", run.stderr);
        for (line, source) in lines
            .iter()
            .zip(["path:../home/dotfiles/claude", "path:./via"])
        {
            assert_eq!(
                *line,
                format!(
                    "error [source] layer '{source}' touches an account folder or the home folder; alz never reads there"
                )
            );
        }
    }

    #[test]
    fn a_local_git_path_with_dot_dot_and_a_file_url_without_a_path_are_refused() {
        let world = World::new();
        plant_git_spy(&world);
        world.manifest(
            "stack:\n  - source: git:../work/../work@main\n  - source: git:file://host@main\n",
        );
        let run = world.alz();
        assert_eq!(run.code, 1);
        let lines: Vec<&str> = run.stderr.lines().collect();
        assert_eq!(lines.len(), 2, "{}", run.stderr);
        assert!(lines[0].starts_with("error [source] "), "{}", lines[0]);
        assert!(
            lines[0].contains("'git:../work/../work@main'"),
            "{}",
            lines[0]
        );
        assert!(lines[1].starts_with("error [source] "), "{}", lines[1]);
        assert!(lines[1].contains("'git:file://host@main'"), "{}", lines[1]);
        assert!(!world.scratch.exists("git-ran"));
    }

    #[test]
    fn a_cache_that_cannot_be_located_is_reported_once_for_all_the_entries_that_need_it() {
        let world = World::new();
        world.manifest(
            "stack:\n  - source: git:file:///nowhere/a@main\n  - source: github:o/r@main\n",
        );
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(world.path("repo"))
            .env_clear()
            .env("PATH", real_path())
            .env("HOME", "relative")
            .arg("lint")
            .output()
            .expect("alz runs");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            stderr,
            "error [cache] cannot locate the layer cache: set HOME or XDG_CACHE_HOME to an absolute path\n"
        );
    }
}
