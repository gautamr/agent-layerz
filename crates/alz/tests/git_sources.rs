//! `git:` and `github:` layers in the layer cache (spec §5.3 "Cache integrity", §6 "A failed
//! fetch"; TASK-55.4.8, TASK-68): real git in temporary repositories as the remotes, `file://` and
//! path sources for the local clone, and a fake `ssh` for the remote one. Each test builds its own
//! machine, with a home folder, a repository, a source repository and a cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use alz_core::account::AccountFolders;
    use alz_core::cache::{CacheLocation, EntryName};
    use alz_core::env::EnvVars;
    use alz_core::lock::{ContentHash, LayerFiles, LockedLayer, ObjectId};
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::stack::{CloneOrigin, GitRef, GitUrl, OriginRefusal, Source, github_origin};
    use alz_core::{Diagnostic, layer::LayerName};
    use alz_io::{
        CacheDir, Deadlines, EnvSnapshot, FetchedLayer, Fetcher, GitLayer, GitRunner, Launcher,
        ReadHow, RepositoryPaths, locate_git_source,
    };
    use alz_test_support::ScratchDir;
    use assert_cmd::Command;

    /// A fake `ssh`: the remote repository is `$ALZ_FAKE_REMOTE`, whatever git asks for.
    const SSH_TO_THE_REMOTE: &str = "#!/bin/sh\nexec git upload-pack \"$ALZ_FAKE_REMOTE\"\n";

    /// A fake `ssh` that writes control characters and fails.
    const SSH_THAT_FAILS_LOUDLY: &str =
        "#!/bin/sh\nprintf '\\033[31mboom\\r\\n' >&2\nprintf 'second line\\n' >&2\nexit 255\n";

    /// A fake `ssh` that never answers.
    const SSH_THAT_SLEEPS: &str = "#!/bin/sh\nexec sleep 300\n";

    /// A `git` in front of the real one: it can fail a checkout, and it runs a command after one.
    const WRAPPED_GIT: &str = r#"#!/bin/sh
if [ "$3" = checkout ] && [ -n "$ALZ_FAIL_CHECKOUT" ]; then
  echo "fatal: checkout exploded" >&2
  exit 1
fi
PATH="$ALZ_REAL_PATH" git "$@"
status=$?
if [ "$3" = checkout ] && [ -n "$ALZ_AFTER_CHECKOUT" ]; then eval "$ALZ_AFTER_CHECKOUT"; fi
exit $status
"#;

    /// One machine: everything lies under the real path of one scratch folder.
    struct World {
        scratch: ScratchDir,
        root: PathBuf,
    }

    struct Parts {
        source: Source,
        git_ref: GitRef,
        origin: CloneOrigin,
        runner: GitRunner,
        location: CacheLocation,
        cache: CacheDir,
    }

    /// What differs between the runs of one test.
    #[derive(Default)]
    struct Setup {
        extra_env: Vec<(&'static str, OsString)>,
        deadlines: Option<Deadlines>,
    }

    fn absolute(path: &Path) -> AbsolutePath {
        AbsolutePath::try_from(path.to_str().expect("text")).expect("absolute")
    }

    fn real_path() -> OsString {
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
            for name in ["home", "repo", "work", "bin"] {
                scratch.make_dir(name).expect("made");
            }
            Self { scratch, root }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        /// `path`, which lies under the root, as a name inside the scratch directory.
        fn name(&self, path: &Path) -> PathBuf {
            path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
        }

        fn put(&self, path: &Path, text: &str) {
            self.scratch.write(self.name(path), text).expect("written");
        }

        fn text(&self, path: &Path) -> String {
            self.scratch.read_text(self.name(path)).expect("read")
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.scratch.is_dir(self.name(path))
        }

        fn is_file(&self, path: &Path) -> bool {
            self.scratch.is_file(self.name(path))
        }

        fn exists(&self, path: &Path) -> bool {
            self.scratch.exists(self.name(path))
        }

        fn rename(&self, from: &Path, to: &Path) {
            self.scratch
                .rename(self.name(from), self.name(to))
                .expect("renamed");
        }

        fn link(&self, at: &Path, target: &Path) {
            self.scratch
                .make_link(self.name(at), target)
                .expect("linked");
        }

        fn mkdirs(&self, path: &Path) {
            self.scratch.make_dirs(self.name(path)).expect("made");
        }

        fn mode(&self, path: &Path, mode: u32) {
            self.scratch
                .set_mode(self.name(path), mode)
                .expect("mode set");
        }

        fn cache_root(&self) -> PathBuf {
            self.path("xdg/agent-layerz")
        }

        fn work(&self) -> PathBuf {
            self.path("work")
        }

        /// `git` with a fixed identity, in `directory`, giving what it printed.
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

        /// Commits `files` in the source repository and gives the commit.
        fn commit(&self, files: &[(&str, &str)]) -> ObjectId {
            let work = self.work();
            if !self.scratch.exists("work/.git") {
                self.git(&work, &["init", "-q"]);
            }
            for (name, text) in files {
                self.scratch
                    .write(format!("work/{name}"), text)
                    .expect("written");
            }
            self.git(&work, &["add", "-A"]);
            self.git(&work, &["commit", "-q", "-m", "commit"]);
            ObjectId::try_from(self.git(&work, &["rev-parse", "HEAD"]).as_str()).expect("an id")
        }

        fn source_of_work(&self, git_ref: &str) -> String {
            format!("git:file://{}@{git_ref}", self.work().display())
        }

        /// A bare copy of the source repository, which can then be removed: a work tree's clone
        /// argument is its `.git` folder, which a removed source no longer has.
        fn source_of_bare_copy(&self, git_ref: &str) -> String {
            let bare = self.path("bare.git");
            self.git(
                &self.root,
                &[
                    "clone",
                    "-q",
                    "--bare",
                    self.work().to_str().unwrap(),
                    bare.to_str().unwrap(),
                ],
            );
            format!("git:file://{}@{git_ref}", bare.display())
        }

        fn remove_bare_copy(&self) {
            self.scratch.remove_tree("bare.git").expect("removed");
        }

        fn executable(&self, relative: &str, text: &str) {
            self.scratch
                .write_executable(relative, text)
                .expect("written");
        }

        fn folders(&self) -> AccountFolders {
            AccountFolders::from_env(&absolute(&self.path("home")), None, None)
        }

        fn location(&self) -> CacheLocation {
            CacheLocation::from_env(self.path("home").to_str(), self.path("xdg").to_str())
                .expect("a location")
        }

        fn origin_of(&self, source: &Source) -> CloneOrigin {
            match source {
                Source::Git { url, .. } => {
                    locate_git_source(url, &absolute(&self.path("repo")), &self.folders())
                        .expect("an origin")
                }
                Source::GitHub { owner, repo, .. } => github_origin(owner, repo),
                Source::Path { .. } => panic!("a path: source is not cloned"),
            }
        }

        fn snapshot(&self, setup: &Setup) -> EnvSnapshot {
            let path = {
                let mut path = self.path("bin").into_os_string();
                path.push(":");
                path.push(real_path());
                path
            };
            let mut pairs: Vec<(OsString, OsString)> = vec![
                ("PATH".into(), path),
                ("HOME".into(), self.path("home").into_os_string()),
                ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
                ("ALZ_REAL_PATH".into(), real_path()),
                ("ALZ_FAKE_REMOTE".into(), self.work().into_os_string()),
            ];
            pairs.extend(
                setup
                    .extra_env
                    .iter()
                    .map(|(name, value)| ((*name).into(), value.clone())),
            );
            EnvSnapshot::from_vars(pairs.into_iter().collect::<EnvVars>())
        }

        /// What a command has built before it resolves `source`: the source, the origin, a
        /// runner, the cache and where it is.
        fn parts(&self, source: &str, setup: &Setup) -> Parts {
            let source = Source::try_from(source).expect("a source");
            let git_ref = match &source {
                Source::Git { git_ref, .. } | Source::GitHub { git_ref, .. } => git_ref.clone(),
                Source::Path { .. } => panic!("a path: source is not cloned"),
            };
            let origin = self.origin_of(&source);
            let runner = GitRunner::with_launcher(
                Launcher::at(env!("CARGO_BIN_EXE_alz")),
                &self.snapshot(setup),
                setup.deadlines.unwrap_or(Deadlines::STANDARD),
            )
            .expect("a git");
            let location = self.location();
            let cache = CacheDir::open_or_create(
                &location,
                &self.folders(),
                &RepositoryPaths::new(absolute(&self.path("repo")), absolute(&self.path("repo"))),
            )
            .expect("a cache");
            Parts {
                source,
                git_ref,
                origin,
                runner,
                location,
                cache,
            }
        }

        /// Resolves `source`, as a command would.
        fn fetch(
            &self,
            source: &str,
            locked: Option<&LockedLayer>,
            setup: &Setup,
        ) -> Result<FetchedLayer, Diagnostic> {
            let parts = self.parts(source, setup);
            Fetcher::new(&parts.cache, &parts.location, &parts.runner)
                .resolve(&GitLayer {
                    source: &parts.source,
                    git_ref: &parts.git_ref,
                    origin: &parts.origin,
                    locked,
                })
                .map_err(|diagnostics| {
                    diagnostics
                        .into_iter()
                        .next()
                        .expect("a failed fetch has a diagnostic")
                })
        }

        fn entry_path(&self, source: &str, commit: ObjectId) -> PathBuf {
            let origin = self.origin_of(&Source::try_from(source).expect("a source"));
            self.cache_root().join(
                EntryName::Git {
                    key: origin.key(),
                    commit,
                }
                .to_string(),
            )
        }

        /// What is in the cache root.
        fn cache_names(&self) -> Vec<String> {
            self.scratch.list("xdg/agent-layerz")
        }
    }

    fn rel(text: &str) -> SafeRelPath {
        SafeRelPath::try_from(text).expect("a path")
    }

    fn files(contents: &[(&str, &str)]) -> LayerFiles {
        LayerFiles::from_contents(
            contents
                .iter()
                .map(|(path, text)| (rel(path), text.as_bytes())),
        )
    }

    fn locked(source: &str, sha: ObjectId, held: &LayerFiles) -> LockedLayer {
        LockedLayer {
            name: LayerName::try_from("team").expect("a name"),
            source: Source::try_from(source).expect("a source"),
            rev: Some(sha.short_rev()),
            sha: Some(sha),
            content_hash: ContentHash::of(held),
        }
    }

    const LAYER: &[(&str, &str)] = &[("layer.yaml", "name: team\n"), ("a.md", "hello\n")];

    fn error_text(result: Result<FetchedLayer, Diagnostic>) -> String {
        result.expect_err("a failure").to_string()
    }

    // --- AC #1: cloned once, read from the cache when the lock pins it --------------------------

    #[test]
    fn a_git_layer_is_cloned_once_into_its_entry_with_the_twelve_character_rev() {
        let world = World::new();
        let sha = world.commit(&[
            ("layer.yaml", "name: team\n"),
            ("a.md", "hello\n"),
            (".env.dev", "SECRET=1\n"),
        ]);
        let source = world.source_of_work("main");

        let fetched = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");

        assert_eq!(fetched.read, ReadHow::Fetched);
        assert_eq!(fetched.sha, sha);
        assert_eq!(
            fetched.rev.as_str(),
            sha.to_string().chars().take(12).collect::<String>()
        );
        assert_eq!(fetched.contents.files(), files(LAYER));
        assert_eq!(fetched.env_files, [rel(".env.dev")]);
        let entry = world.entry_path(&source, sha);
        assert!(world.is_file(&entry.join("layer.yaml")));
        assert_eq!(world.text(&entry.join("a.md")), "hello\n");
        // The clone went through a temporary entry, which no longer exists.
        assert_eq!(
            world.cache_names(),
            [entry.file_name().unwrap().to_string_lossy()]
        );
    }

    #[test]
    fn a_layer_the_lock_pins_is_read_from_the_cache_without_cloning_again() {
        let world = World::new();
        let sha = world.commit(LAYER);
        let source = world.source_of_bare_copy("main");
        let first = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");
        let pin = locked(&source, sha, &first.contents.files());
        // With the source gone, only the cache can answer.
        world.remove_bare_copy();

        let again = world
            .fetch(&source, Some(&pin), &Setup::default())
            .expect("read");

        assert_eq!(again.read, ReadHow::Cached);
        assert_eq!(again.sha, sha);
        assert_eq!(again.rev, sha.short_rev());
        assert_eq!(again.contents.files(), first.contents.files());
    }

    #[test]
    fn a_pin_the_cache_lacks_is_cloned_at_the_pinned_commit_though_the_ref_moved() {
        let world = World::new();
        let pinned = world.commit(LAYER);
        let moved = world.commit(&[("a.md", "changed\n")]);
        assert_ne!(pinned, moved);
        let source = world.source_of_work("main");
        let pin = locked(&source, pinned, &files(LAYER));

        let fetched = world
            .fetch(&source, Some(&pin), &Setup::default())
            .expect("fetched");

        assert_eq!(fetched.read, ReadHow::Fetched);
        assert_eq!(fetched.sha, pinned);
        assert_eq!(fetched.contents.files(), files(LAYER));
        assert!(world.is_dir(&world.entry_path(&source, pinned)));
        assert!(!world.exists(&world.entry_path(&source, moved)));
    }

    #[test]
    fn without_a_pin_a_moved_ref_gives_the_new_commit_in_a_new_entry() {
        let world = World::new();
        let first = world.commit(LAYER);
        let source = world.source_of_work("main");
        world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");
        let second = world.commit(&[("a.md", "changed\n")]);

        let fetched = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");

        assert_eq!(fetched.sha, second);
        assert!(world.is_dir(&world.entry_path(&source, first)));
        assert!(world.is_dir(&world.entry_path(&source, second)));
    }

    #[test]
    fn a_remote_source_is_cloned_through_ssh_and_its_entry_is_named_after_the_url() {
        let world = World::new();
        let sha = world.commit(LAYER);
        world.executable("bin/ssh", SSH_TO_THE_REMOTE);
        let source = "git:ssh://example.invalid/team/layer.git@main";
        let setup = Setup {
            extra_env: vec![("GIT_SSH_COMMAND", world.path("bin/ssh").into_os_string())],
            ..Setup::default()
        };

        let fetched = world.fetch(source, None, &setup).expect("fetched");

        assert_eq!(fetched.sha, sha);
        assert_eq!(fetched.contents.files(), files(LAYER));
        let name = EntryName::Git {
            key: alz_core::cache::CloneKey::of("ssh://example.invalid/team/layer.git"),
            commit: sha,
        };
        assert!(world.is_dir(&world.cache_root().join(name.to_string())));
    }

    // --- AC #2: a changed entry and a link are the cache error ------------------------------------

    #[test]
    fn an_entry_that_differs_from_the_lock_stops_with_the_cache_error_before_any_git_call() {
        let world = World::new();
        let sha = world.commit(LAYER);
        let source = world.source_of_bare_copy("main");
        let first = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");
        let pin = locked(&source, sha, &first.contents.files());
        let entry = world.entry_path(&source, sha);
        world.put(&entry.join("a.md"), "tampered\n");
        world.remove_bare_copy();

        let message = error_text(world.fetch(&source, Some(&pin), &Setup::default()));

        assert_eq!(
            message,
            format!(
                "error [cache] cached layer 'team' ({source}) no longer matches layer.lock: \
                 its files in {} changed after they were cached; delete that directory and rebuild",
                entry.display()
            )
        );
    }

    #[test]
    fn a_file_added_to_a_cached_entry_is_a_difference_too() {
        let world = World::new();
        let sha = world.commit(LAYER);
        let source = world.source_of_work("main");
        let first = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");
        let pin = locked(&source, sha, &first.contents.files());
        world.put(&world.entry_path(&source, sha).join("extra.md"), "x");

        let message = error_text(world.fetch(&source, Some(&pin), &Setup::default()));

        assert!(
            message.starts_with("error [cache] cached layer 'team'"),
            "{message}"
        );
    }

    #[test]
    fn an_entry_that_is_a_link_is_refused_with_and_without_a_lock() {
        let world = World::new();
        let sha = world.commit(LAYER);
        let source = world.source_of_work("main");
        let first = world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");
        let pin = locked(&source, sha, &first.contents.files());
        let entry = world.entry_path(&source, sha);
        world.rename(&entry, &world.path("elsewhere"));
        world.link(&entry, &world.path("elsewhere"));
        let expected = format!(
            "error [cache] cache entry {} in the layer cache at {} must be a folder, not a link \
             or a file; delete it and run alz again",
            entry.file_name().unwrap().to_string_lossy(),
            world.cache_root().display()
        );

        assert_eq!(
            error_text(world.fetch(&source, Some(&pin), &Setup::default())),
            expected
        );
        assert_eq!(
            error_text(world.fetch(&source, None, &Setup::default())),
            expected
        );
        // Nothing was written through the link, and no temporary entry stays.
        assert!(world.is_file(&world.path("elsewhere/layer.yaml")));
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-"))
        );
    }

    #[test]
    fn an_existing_entry_is_compared_with_a_fresh_clone_when_the_lock_has_no_entry() {
        let world = World::new();
        let sha = world.commit(LAYER);
        let source = world.source_of_work("main");
        world
            .fetch(&source, None, &Setup::default())
            .expect("fetched");

        let same = world
            .fetch(&source, None, &Setup::default())
            .expect("equal");
        assert_eq!(same.sha, sha);
        assert_eq!(same.contents.files(), files(LAYER));

        let entry = world.entry_path(&source, sha);
        world.put(&entry.join("a.md"), "tampered\n");
        let message = error_text(world.fetch(&source, None, &Setup::default()));

        assert_eq!(
            message,
            format!(
                "error [cache] cached layer '{source}' differs from a fresh clone of the same \
                 commit: its files in {} changed after they were cached; delete that directory \
                 and rebuild",
                entry.display()
            )
        );
        // The clone was discarded, and the entry was left as it was found.
        assert_eq!(world.text(&entry.join("a.md")), "tampered\n");
        assert_eq!(
            world.cache_names(),
            [entry.file_name().unwrap().to_string_lossy()]
        );
    }

    // --- AC #3: failed git steps are `source`, failed stores are `cache` --------------------------

    #[test]
    fn a_clone_that_fails_is_cannot_fetch_with_the_first_line_of_git_s_standard_error() {
        let world = World::new();
        let source = format!("git:{}/missing@v1", world.root.display());

        let message = error_text(world.fetch(&source, None, &Setup::default()));

        let prefix = format!("error [source] cannot fetch '{source}': ");
        assert!(message.starts_with(&prefix), "{message}");
        let line = message.strip_prefix(&prefix).expect("the prefix");
        assert!(line.contains("does not exist"), "{line}");
        assert!(!line.contains('\n'), "{line}");
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-"))
        );
    }

    #[test]
    fn what_a_remote_writes_to_standard_error_reaches_the_message_without_control_characters() {
        let world = World::new();
        world.executable("bin/ssh", SSH_THAT_FAILS_LOUDLY);
        let source = "git:ssh://example.invalid/team/layer.git@main";
        let setup = Setup {
            extra_env: vec![("GIT_SSH_COMMAND", world.path("bin/ssh").into_os_string())],
            ..Setup::default()
        };

        let message = error_text(world.fetch(source, None, &setup));

        assert_eq!(
            message,
            format!("error [source] cannot fetch '{source}': [31mboom")
        );
    }

    #[test]
    fn a_lock_sha_that_is_not_a_commit_is_cannot_fetch_without_a_line() {
        let world = World::new();
        world.commit(LAYER);
        let source = world.source_of_work("main");
        let not_a_commit = ObjectId::try_from("0123456789abcdef0123456789abcdef01234567").unwrap();
        let pin = locked(&source, not_a_commit, &files(LAYER));

        let message = error_text(world.fetch(&source, Some(&pin), &Setup::default()));

        assert_eq!(message, format!("error [source] cannot fetch '{source}'"));
        assert!(!world.exists(&world.entry_path(&source, not_a_commit)));
    }

    #[test]
    fn a_ref_that_is_not_a_commit_is_cannot_fetch_from_the_rev_parse_step() {
        let world = World::new();
        world.commit(LAYER);
        let source = world.source_of_work("no-such-ref");

        let message = error_text(world.fetch(&source, None, &Setup::default()));

        assert_eq!(message, format!("error [source] cannot fetch '{source}'"));
    }

    #[test]
    fn a_checkout_that_fails_is_cannot_fetch_with_its_first_line() {
        let world = World::new();
        world.commit(LAYER);
        world.executable("bin/git", WRAPPED_GIT);
        let source = world.source_of_work("main");
        let setup = Setup {
            extra_env: vec![("ALZ_FAIL_CHECKOUT", "1".into())],
            ..Setup::default()
        };

        let message = error_text(world.fetch(&source, None, &setup));

        assert_eq!(
            message,
            format!("error [source] cannot fetch '{source}': fatal: checkout exploded")
        );
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-"))
        );
    }

    #[test]
    fn a_clone_past_its_deadline_is_cannot_fetch_and_leaves_no_temporary_entry() {
        let world = World::new();
        world.executable("bin/ssh", SSH_THAT_SLEEPS);
        let source = "git:ssh://example.invalid/team/layer.git@main";
        let setup = Setup {
            extra_env: vec![("GIT_SSH_COMMAND", world.path("bin/ssh").into_os_string())],
            deadlines: Some(Deadlines {
                clone: Duration::from_secs(2),
                other: Duration::from_secs(60),
            }),
        };

        let message = error_text(world.fetch(source, None, &setup));

        assert_eq!(
            message,
            format!("error [source] cannot fetch '{source}': git did not finish within 2 seconds")
        );
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-"))
        );
    }

    fn cannot_store(world: &World, source: &str) -> String {
        format!(
            "error [cache] cannot store '{source}' in the layer cache at {}",
            world.cache_root().display()
        )
    }

    #[test]
    fn a_cache_root_that_cannot_be_written_is_cannot_store_and_not_cannot_fetch() {
        let world = World::new();
        world.commit(LAYER);
        let source = world.source_of_work("main");
        world.mkdirs(&world.cache_root());
        world.mode(&world.cache_root(), 0o500);

        let message = error_text(world.fetch(&source, None, &Setup::default()));
        world.mode(&world.cache_root(), 0o700);

        assert_eq!(message, cannot_store(&world, &source));
    }

    #[test]
    fn a_cache_root_made_read_only_before_the_publish_is_cannot_store() {
        // The superuser ignores modes, so the failure this test makes cannot happen.
        if alz_test_support::running_as_root() {
            return;
        }
        let world = World::new();
        world.commit(LAYER);
        world.executable("bin/git", WRAPPED_GIT);
        let source = world.source_of_work("main");
        let setup = Setup {
            extra_env: vec![(
                "ALZ_AFTER_CHECKOUT",
                format!("chmod 500 '{}'", world.cache_root().display()).into(),
            )],
            ..Setup::default()
        };

        let message = error_text(world.fetch(&source, None, &setup));
        world.mode(&world.cache_root(), 0o700);

        assert_eq!(message, cannot_store(&world, &source));
    }

    #[test]
    fn a_clone_that_cannot_be_read_is_cannot_store_and_not_cannot_fetch() {
        // The superuser ignores modes, so the failure this test makes cannot happen.
        if alz_test_support::running_as_root() {
            return;
        }
        let world = World::new();
        world.commit(LAYER);
        world.executable("bin/git", WRAPPED_GIT);
        let source = world.source_of_work("main");
        let setup = Setup {
            extra_env: vec![("ALZ_AFTER_CHECKOUT", "chmod 000 layer.yaml".into())],
            ..Setup::default()
        };

        let message = error_text(world.fetch(&source, None, &setup));

        assert_eq!(message, cannot_store(&world, &source));
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-"))
        );
    }

    // --- Where a `git:` source is cloned from ------------------------------------------------------

    #[test]
    fn a_local_git_source_is_cloned_from_its_canonical_path_and_a_found_folder_wins_over_a_url() {
        let world = World::new();
        world.mkdirs(&world.path("repo/x:y"));
        world.link(&world.path("repo/link"), &world.path("repo/x:y"));
        let folders = world.folders();
        let locate = |url: &str| {
            let url = GitUrl::try_from(url).expect("a url");
            locate_git_source(&url, &absolute(&world.path("repo")), &folders)
        };

        let found = locate("x:y").expect("local");
        assert_eq!(found.argument(), world.path("repo/x:y").to_str().unwrap());
        assert_eq!(found.transport(), alz_core::git::CloneTransport::Local);
        assert_eq!(
            locate("link").expect("local").argument(),
            world.path("repo/x:y").to_str().unwrap()
        );
        assert_eq!(
            locate("other:y").expect("remote").transport(),
            alz_core::git::CloneTransport::Remote
        );
        assert_eq!(locate("../repo/x:y").err(), Some(OriginRefusal::DotDot));
    }

    /// A file name with the byte 0xFF cannot be made on macOS: APFS refuses it with `EILSEQ`
    /// (spec §5.3 "File names"). The clone's checkout can create it only on Linux, so this case
    /// runs there. The commit is made with plumbing (`mktree -z`), so the name is any bytes.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_clone_that_holds_a_name_that_is_not_utf8_is_the_layer_error_and_publishes_nothing() {
        let world = World::new();
        let work = world.work();
        world.mkdirs(&work);
        world.git(&work, &["init", "-q"]);
        let plumbing = |args: &[&str], input: &[u8]| -> String {
            let output = Command::new("git")
                .current_dir(&work)
                .env_clear()
                .env("PATH", real_path())
                .env("HOME", world.path("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
                .args(args)
                .write_stdin(input)
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {args:?}: {output:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        let blob = plumbing(&["hash-object", "-w", "--stdin"], b"x\n");
        let record =
            |name: &[u8]| [format!("100644 blob {blob}\t").as_bytes(), name, b"\0"].concat();
        let tree = plumbing(
            &["mktree", "-z"],
            &[record(b"ok.md"), record(b"bad\xff.md")].concat(),
        );
        let commit = plumbing(&["commit-tree", &tree, "-m", "raw"], b"");
        world.git(&work, &["update-ref", "refs/heads/main", &commit]);
        let source = world.source_of_work("main");

        let text = error_text(world.fetch(&source, None, &Setup::default()));

        assert_eq!(
            text,
            format!(
                "error [layer] a file or folder name in layer '{source}' is not valid UTF-8; rename it"
            )
        );
        assert!(
            !text.contains('\u{fffd}') && !text.as_bytes().contains(&0xff),
            "{text}"
        );
        assert!(
            world
                .cache_names()
                .iter()
                .all(|name| !name.starts_with(".tmp-") && !name.starts_with("git-")),
            "{:?}",
            world.cache_names()
        );
    }

    #[test]
    fn a_local_git_source_in_an_account_folder_is_refused_before_it_is_looked_at() {
        let world = World::new();
        world.mkdirs(&world.path("home/.claude-work/hub"));
        world.link(&world.path("repo/sneaky"), &world.path("home/.claude-work"));
        let folders = world.folders();
        let locate = |url: &str| {
            let url = GitUrl::try_from(url).expect("a url");
            locate_git_source(&url, &absolute(&world.path("repo")), &folders)
        };

        assert_eq!(
            locate(world.path("home/.claude-work/hub").to_str().unwrap()).err(),
            Some(OriginRefusal::AccountFolder)
        );
        assert_eq!(
            locate("sneaky/hub").err(),
            Some(OriginRefusal::AccountFolder)
        );
    }
}
