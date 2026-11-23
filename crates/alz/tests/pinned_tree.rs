//! Pinned `path:` layers in the layer cache (spec §5.3 "Cache integrity", §6 "A pinned `path:`
//! layer"; TASK-55.4.9, TASK-69): real git in temporary repositories. The entry `tree2@<tree>` is
//! written from the tree's blobs, never from a checkout or the working tree, and is verified
//! against `git ls-tree -r` before it is published and on every later read.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};

    use alz_core::account::AccountFolders;
    use alz_core::cache::{CacheLocation, DirKey, EntryName};
    use alz_core::env::EnvVars;
    use alz_core::git::{BlobId, ObjectFormat};
    use alz_core::layer::LayerName;
    use alz_core::lock::{ContentHash, LayerFiles, LockedLayer, ObjectId};
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::stack::{GitRef, Source};
    use alz_core::{Diagnostic, Level};
    use alz_io::{
        CacheDir, Deadlines, EnvSnapshot, GitRunner, Launcher, PinnedLayer, PinnedRead,
        PinnedTreeReader, ReadHow, RepositoryPaths,
    };
    use alz_test_support::ScratchDir;
    use assert_cmd::Command;

    /// One machine: a home folder, a repository being built, a hub repository whose directory
    /// `layer` is the pinned layer, and a cache. Everything lies under one real path.
    struct World {
        scratch: ScratchDir,
        root: PathBuf,
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

    fn source_at(dir: &Path) -> Source {
        Source::try_from(format!("path:{}", dir.display()).as_str()).expect("a source")
    }

    fn ref_of(text: &str) -> GitRef {
        GitRef::try_from(text).expect("a ref")
    }

    fn rel(text: &str) -> SafeRelPath {
        SafeRelPath::try_from(text).expect("a path")
    }

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).expect("an id")
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let root = scratch.real_path().expect("a real path");
            for name in ["home", "repo", "hub"] {
                scratch.make_dir(name).expect("made");
            }
            Self { scratch, root }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn name(&self, path: &Path) -> PathBuf {
            path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
        }

        fn hub(&self) -> PathBuf {
            self.path("hub")
        }

        fn cache_root(&self) -> PathBuf {
            self.path("xdg/agent-layerz")
        }

        fn put(&self, path: &Path, text: &str) {
            self.scratch.write(self.name(path), text).expect("written");
        }

        fn text(&self, path: &Path) -> String {
            self.scratch.read_text(self.name(path)).expect("read")
        }

        fn command(&self, directory: &Path) -> Command {
            let mut command = Command::new("git");
            command
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
                ]);
            command
        }

        /// `git` in `directory`, giving what it printed.
        fn git_in(&self, directory: &Path, args: &[&OsStr], stdin: &[u8]) -> String {
            let output = self
                .command(directory)
                .args(args)
                .write_stdin(stdin.to_vec())
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {args:?}: {output:?}");
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        }

        fn git(&self, args: &[&str]) -> String {
            let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
            self.git_in(&self.hub(), &args, b"")
        }

        /// Commits `files` (path below the hub, text) and gives the commit.
        fn commit(&self, files: &[(&str, &str)]) -> ObjectId {
            self.ensure_repository();
            for (name, text) in files {
                let at = self.hub().join(name);
                if let Some(parent) = at.parent() {
                    self.scratch.make_dirs(self.name(parent)).expect("made");
                }
                self.put(&at, text);
            }
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "-m", "commit"]);
            id(&self.git(&["rev-parse", "HEAD"]))
        }

        /// Makes `hub/layer` hold exactly `records` (NUL-ended `mktree -z` records, so a name can
        /// be anything git accepts) and commits it as `main`, without a working tree.
        fn commit_raw_layer(&self, records: &[u8]) -> ObjectId {
            self.ensure_repository();
            self.scratch.make_dirs("hub/layer").expect("made");
            let hub = self.hub();
            let layer = self.git_in(&hub, &[OsStr::new("mktree"), OsStr::new("-z")], records);
            let root = self.git_in(
                &hub,
                &[OsStr::new("mktree")],
                format!("040000 tree {layer}\tlayer\n").as_bytes(),
            );
            let commit = self.git_in(
                &hub,
                &[
                    OsStr::new("commit-tree"),
                    OsStr::new(&root),
                    OsStr::new("-m"),
                    OsStr::new("raw"),
                ],
                b"",
            );
            self.git(&["update-ref", "refs/heads/main", &commit]);
            id(&commit)
        }

        fn ensure_repository(&self) {
            if !self.scratch.exists("hub/.git") {
                self.git(&["init", "-q"]);
            }
        }

        fn blob(&self, text: &str) -> String {
            self.ensure_repository();
            self.git_in(
                &self.hub(),
                &[
                    OsStr::new("hash-object"),
                    OsStr::new("-w"),
                    OsStr::new("--stdin"),
                ],
                text.as_bytes(),
            )
        }

        fn tree_of(&self, commit: ObjectId, path: &str) -> ObjectId {
            id(&self.git(&["rev-parse", &format!("{commit}:{path}")]))
        }

        fn folders(&self) -> AccountFolders {
            AccountFolders::from_env(&absolute(&self.path("home")), None, None)
        }

        fn parts(&self, writable_cache: bool) -> (GitRunner, CacheLocation, CacheDir) {
            let pairs: Vec<(OsString, OsString)> = vec![
                ("PATH".into(), real_path()),
                ("HOME".into(), self.path("home").into_os_string()),
                ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ];
            let runner = GitRunner::with_launcher(
                Launcher::at(env!("CARGO_BIN_EXE_alz")),
                &EnvSnapshot::from_vars(pairs.into_iter().collect::<EnvVars>()),
                Deadlines::STANDARD,
            )
            .expect("a git");
            let location =
                CacheLocation::from_env(self.path("home").to_str(), self.path("xdg").to_str())
                    .expect("a location");
            let folders = self.folders();
            let cache = CacheDir::open_or_create(
                &location,
                &folders,
                &RepositoryPaths::new(absolute(&self.path("repo")), absolute(&self.path("repo"))),
            )
            .expect("a cache");
            assert!(writable_cache || self.scratch.exists("xdg/agent-layerz"));
            (runner, location, cache)
        }

        fn resolve_in(
            &self,
            repository: &Path,
            layer: &str,
            git_ref: &str,
            locked: Option<&LockedLayer>,
        ) -> Result<PinnedRead, Diagnostic> {
            let (runner, location, cache) = self.parts(true);
            let dir = repository.join(layer);
            let source = source_at(&dir);
            let git_ref = ref_of(git_ref);
            PinnedTreeReader::new(&cache, &location, &runner).resolve(&PinnedLayer {
                source: &source,
                git_ref: &git_ref,
                real_dir: &absolute(&dir),
                folders: &self.folders(),
                locked,
            })
        }

        fn resolve(&self, git_ref: &str) -> Result<PinnedRead, Diagnostic> {
            self.resolve_in(&self.hub(), "layer", git_ref, None)
        }

        fn entry(&self, tree: ObjectId) -> PathBuf {
            self.cache_root().join(EntryName::Tree(tree).to_string())
        }

        fn index(&self, commit: ObjectId) -> PathBuf {
            self.cache_root().join(
                EntryName::PathIndex {
                    dir: DirKey::of(&absolute(&self.path("hub/layer"))),
                    commit,
                }
                .to_string(),
            )
        }

        fn cache_names(&self) -> Vec<String> {
            self.scratch.list("xdg/agent-layerz")
        }
    }

    fn error_text(result: Result<PinnedRead, Diagnostic>) -> String {
        result.expect_err("a failure").to_string()
    }

    const LAYER: &[(&str, &str)] = &[
        ("layer/layer.yaml", "name: team\n"),
        ("layer/a.md", "hello\n"),
        ("layer/docs/b.md", "world\n"),
    ];

    // --- AC #1: written from blobs, verified, indexed -------------------------------------------

    #[test]
    fn a_pinned_layer_is_written_to_its_tree_entry_from_the_committed_blobs() {
        let world = World::new();
        world.commit(&[
            (".gitattributes", "* text eol=crlf\n"),
            ("layer/.gitattributes", "*.md text eol=crlf\n"),
            ("layer/layer.yaml", "name: team\n"),
            ("layer/a.md", "hello\n"),
            ("layer/run.sh", "#!/bin/sh\n"),
            ("layer/.env.dev", "SECRET=1\n"),
        ]);
        world.git(&["update-index", "--chmod=+x", "layer/run.sh"]);
        world.git(&["commit", "-q", "-m", "mode"]);
        let commit = id(&world.git(&["rev-parse", "HEAD"]));
        let tree = world.tree_of(commit, "layer");
        // The working tree is not read: what is committed is what is written.
        world.put(
            &world.hub().join("layer/a.md"),
            "changed in the work tree\n",
        );
        world.put(&world.hub().join("layer/untracked.md"), "never committed\n");

        let read = world.resolve("main").expect("resolved");

        assert_eq!(read.layer.read, ReadHow::Fetched);
        assert_eq!(read.layer.sha, commit);
        assert_eq!(read.layer.rev.as_str(), "main");
        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        assert_eq!(read.warnings.len(), 0);
        let entry = world.entry(tree);
        // No eol or attributes conversion: the bytes are the blob's.
        assert_eq!(world.text(&entry.join("a.md")), "hello\n");
        assert_eq!(
            world.text(&entry.join(".gitattributes")),
            "*.md text eol=crlf\n"
        );
        assert!(
            !world
                .scratch
                .exists(world.name(&entry.join("untracked.md")))
        );
        assert!(!world.scratch.exists(world.name(&entry.join(".git"))));
        assert_eq!(
            read.layer.files,
            LayerFiles::from_contents(
                [
                    (".gitattributes", "*.md text eol=crlf\n"),
                    ("a.md", "hello\n"),
                    ("layer.yaml", "name: team\n"),
                    ("run.sh", "#!/bin/sh\n"),
                ]
                .iter()
                .map(|(path, text)| (rel(path), text.as_bytes()))
            )
        );
        assert_eq!(read.layer.env_files, [rel(".env.dev")]);
        let modes = |name: &str| world.scratch.mode(world.name(&entry.join(name))).unwrap();
        assert_eq!(modes("a.md"), 0o644);
        assert_eq!(modes("run.sh"), 0o755);
        // Only the entry and the index are in the cache: the temporary folder is gone.
        let mut expected = vec![
            EntryName::PathIndex {
                dir: DirKey::of(&absolute(&world.path("hub/layer"))),
                commit,
            }
            .to_string(),
            EntryName::Tree(tree).to_string(),
        ];
        expected.sort();
        assert_eq!(world.cache_names(), expected);
    }

    #[test]
    fn a_ref_and_the_commit_sha_name_the_same_entry() {
        let world = World::new();
        let commit = world.commit(LAYER);
        world.git(&["tag", "v1"]);
        let by_tag = world.resolve("v1").expect("by tag");
        let by_sha = world.resolve(&commit.to_string()).expect("by sha");
        assert_eq!(by_tag.layer.entry, by_sha.layer.entry);
        assert_eq!(by_sha.layer.sha, commit);
        assert_eq!(by_sha.layer.rev.as_str(), commit.to_string());
        assert_eq!(by_sha.layer.read, ReadHow::Cached);
    }

    #[test]
    fn a_docs_commit_moves_the_commit_but_not_the_tree_and_the_entry_is_still_verified() {
        let world = World::new();
        let first = world.commit(LAYER);
        let tree = world.tree_of(first, "layer");
        world.resolve("main").expect("first");
        let second = world.commit(&[("README.md", "docs only\n")]);
        assert_ne!(first, second);
        assert_eq!(world.tree_of(second, "layer"), tree);

        let read = world.resolve("main").expect("second");

        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        assert_eq!(read.layer.read, ReadHow::Cached);
        assert_eq!(read.layer.sha, second);
        // Each commit has its own index entry, both naming the tree.
        assert_eq!(world.text(&world.index(first)), tree.to_string());
        assert_eq!(world.text(&world.index(second)), tree.to_string());
        // A change to the entry is noticed although the tree did not change.
        world.put(&world.entry(tree).join("a.md"), "edited\n");
        let text = error_text(world.resolve("main"));
        assert!(text.starts_with("error [cache] cached layer "), "{text}");
        assert!(text.contains("delete that directory and rebuild"), "{text}");
    }

    // --- the index -------------------------------------------------------------------------------

    #[test]
    fn the_index_is_written_once_rewritten_when_it_differs_and_never_removed() {
        let world = World::new();
        let commit = world.commit(LAYER);
        let tree = world.tree_of(commit, "layer");
        world.resolve("main").expect("resolved");
        let index = world.index(commit);
        assert_eq!(world.text(&index), tree.to_string());

        world.put(&index, "garbage");
        world.resolve("main").expect("resolved again");
        assert_eq!(world.text(&index), tree.to_string());
    }

    #[test]
    fn a_folder_at_the_index_name_is_the_cache_error() {
        let world = World::new();
        let commit = world.commit(LAYER);
        world.resolve("main").expect("resolved");
        let index = world.index(commit);
        world.scratch.remove_file(world.name(&index)).unwrap();
        world.scratch.make_dir(world.name(&index)).unwrap();
        let text = error_text(world.resolve("main"));
        assert!(
            text.starts_with("error [cache] cache entry pathref@"),
            "{text}"
        );
        assert!(
            text.ends_with("is a folder; delete it and rebuild"),
            "{text}"
        );
    }

    #[test]
    fn an_index_that_cannot_be_written_is_a_warning_and_the_layer_still_resolves() {
        // The superuser ignores modes, so the failure this test makes cannot happen.
        if alz_test_support::running_as_root() {
            return;
        }
        let world = World::new();
        let commit = world.commit(LAYER);
        let tree = world.tree_of(commit, "layer");
        world.resolve("main").expect("resolved");
        world
            .scratch
            .remove_file(world.name(&world.index(commit)))
            .unwrap();
        world
            .scratch
            .set_mode(world.name(&world.cache_root()), 0o500)
            .unwrap();

        let result = world.resolve("main");

        world
            .scratch
            .set_mode(world.name(&world.cache_root()), 0o700)
            .unwrap();
        let read = result.expect("resolved without an index");
        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        let warning = read.warnings.first().expect("a warning");
        assert_eq!(warning.level(), Level::Warning);
        assert!(
            warning.to_string().starts_with("warning [cache-index] "),
            "{warning}"
        );
    }

    // --- verification of an entry ----------------------------------------------------------------

    #[test]
    fn an_entry_that_was_tampered_with_is_refused_on_every_read() {
        type Tamper = fn(&World, &Path);
        let tamperings: [(&str, Tamper); 5] = [
            ("a changed file", |world, entry| {
                world.put(&entry.join("a.md"), "edited\n");
            }),
            ("an extra file", |world, entry| {
                world.put(&entry.join("extra.md"), "x\n");
            }),
            ("a missing file", |world, entry| {
                world
                    .scratch
                    .remove_file(world.name(&entry.join("docs/b.md")))
                    .unwrap();
            }),
            ("a changed mode", |world, entry| {
                world
                    .scratch
                    .set_mode(world.name(&entry.join("a.md")), 0o755)
                    .unwrap();
            }),
            ("a link", |world, entry| {
                world
                    .scratch
                    .make_link(world.name(&entry.join("link.md")), "a.md")
                    .unwrap();
            }),
        ];
        for (what, tamper) in tamperings {
            let world = World::new();
            let commit = world.commit(LAYER);
            let tree = world.tree_of(commit, "layer");
            world.resolve("main").expect("resolved");
            tamper(&world, &world.entry(tree));
            for _ in 0..2 {
                let text = error_text(world.resolve("main"));
                assert!(
                    text.starts_with("error [cache] cached layer ")
                        && text.contains(&format!("no longer matches git tree {tree}"))
                        && text.ends_with("delete that directory and rebuild"),
                    "{what}: {text}"
                );
            }
        }
    }

    #[test]
    fn skipped_names_in_an_entry_are_ignored_unless_the_tree_has_them() {
        let world = World::new();
        let commit = world.commit(LAYER);
        let tree = world.tree_of(commit, "layer");
        world.resolve("main").expect("resolved");
        let entry = world.entry(tree);
        world.put(&entry.join(".DS_Store"), "finder\n");
        world
            .scratch
            .make_dirs(world.name(&entry.join("node_modules")))
            .unwrap();
        world.put(&entry.join("node_modules/x.js"), "x\n");
        world.resolve("main").expect("still verified");

        // A tree that has such a file has it checked, by its path.
        let with_skipped = world.commit(&[("layer/node_modules/y.js", "y\n")]);
        let tree = world.tree_of(with_skipped, "layer");
        world.resolve("main").expect("written");
        world.put(&world.entry(tree).join("node_modules/y.js"), "tampered\n");
        assert!(error_text(world.resolve("main")).contains("no longer matches git tree"));
    }

    #[test]
    fn an_entry_that_is_a_link_is_the_cache_error_and_is_never_followed() {
        let world = World::new();
        let commit = world.commit(LAYER);
        let tree = world.tree_of(commit, "layer");
        world.resolve("main").expect("resolved");
        let entry = world.entry(tree);
        world.scratch.rename(world.name(&entry), "moved").unwrap();
        world
            .scratch
            .make_link(world.name(&entry), world.path("moved"))
            .unwrap();
        let text = error_text(world.resolve("main"));
        assert!(
            text.starts_with("error [cache] cache entry tree2@"),
            "{text}"
        );
        assert!(
            text.contains("must be a folder, not a link or a file"),
            "{text}"
        );
    }

    #[test]
    fn the_lock_is_checked_for_an_entry_read_from_the_cache_when_it_pins_the_commit() {
        let world = World::new();
        let commit = world.commit(LAYER);
        let first = world.resolve("main").expect("written");
        let source = first_source(&world);
        let locked = |sha: ObjectId, hash: ContentHash| LockedLayer {
            name: LayerName::try_from("team").unwrap(),
            source: source.clone(),
            rev: Some(GitRef::try_from("main").unwrap()),
            sha: Some(sha),
            content_hash: hash,
        };
        let good = locked(commit, ContentHash::of(&first.layer.files));
        let read = world
            .resolve_in(&world.hub(), "layer", "main", Some(&good))
            .expect("matches");
        assert_eq!(read.layer.read, ReadHow::Cached);

        let wrong = locked(commit, ContentHash::of(&LayerFiles::default()));
        let text = error_text(world.resolve_in(&world.hub(), "layer", "main", Some(&wrong)));
        assert!(
            text.starts_with("error [cache] cached layer 'team' ("),
            "{text}"
        );
        assert!(text.contains("no longer matches layer.lock"), "{text}");

        // A lock for another commit expects new content: nothing is compared.
        let other = locked(
            id(&"ab".repeat(20)),
            ContentHash::of(&LayerFiles::default()),
        );
        world
            .resolve_in(&world.hub(), "layer", "main", Some(&other))
            .expect("not compared");
    }

    fn first_source(world: &World) -> Source {
        Source::try_from(format!("path:{}", world.hub().join("layer").display()).as_str()).unwrap()
    }

    // --- AC #3: what does not change what is written ---------------------------------------------

    #[test]
    fn a_replace_ref_does_not_swap_what_is_written() {
        let world = World::new();
        let commit = world.commit(LAYER);
        let tree = world.tree_of(commit, "layer");
        let original = world.git(&["rev-parse", &format!("{commit}:layer/a.md")]);
        let other = world.blob("replaced\n");
        world.git(&["replace", &original, &other]);
        // The replacement is in force for ordinary git, so the test means something.
        assert_eq!(world.git(&["cat-file", "-p", &original]), "replaced");

        let read = world.resolve("main").expect("resolved");

        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        assert_eq!(world.text(&world.entry(tree).join("a.md")), "hello\n");
    }

    #[test]
    fn a_link_or_a_submodule_inside_the_tree_is_skipped_and_not_written() {
        let world = World::new();
        let blob = world.blob("hello\n");
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let records = format!(
            "100644 blob {blob}\tAGENTS.md\0120000 blob {}\tCLAUDE.md\0160000 commit {sha}\tvendor\0",
            world.blob("AGENTS.md")
        );
        let commit = world.commit_raw_layer(records.as_bytes());
        let tree = world.tree_of(commit, "layer");

        let read = world.resolve("main").expect("resolved");

        assert_eq!(
            read.layer.files,
            LayerFiles::from_contents([(rel("AGENTS.md"), b"hello\n".as_slice())])
        );
        let entry = world.entry(tree);
        assert!(!world.scratch.exists(world.name(&entry.join("CLAUDE.md"))));
        assert!(!world.scratch.exists(world.name(&entry.join("vendor"))));
        // A link that is planted in the entry afterwards is a change.
        world
            .scratch
            .make_link(world.name(&entry.join("CLAUDE.md")), "AGENTS.md")
            .unwrap();
        assert!(error_text(world.resolve("main")).contains("no longer matches git tree"));
    }

    #[test]
    fn a_link_or_a_submodule_at_the_layer_path_is_a_ref_that_cannot_be_resolved() {
        for kind in ["link", "submodule"] {
            let world = World::new();
            world.commit(&[("other/layer.yaml", "name: other\n")]);
            let link = world.blob("other");
            let entry = match kind {
                "link" => format!("120000 blob {link}\tlayer"),
                _ => "160000 commit 0123456789abcdef0123456789abcdef01234567\tlayer".to_owned(),
            };
            let tree = world.git_in(
                &world.hub(),
                &[OsStr::new("mktree")],
                format!(
                    "{entry}\n040000 tree {}\tother\n",
                    world.tree_of(id(&world.git(&["rev-parse", "HEAD"])), "other")
                )
                .as_bytes(),
            );
            let commit = world.git_in(
                &world.hub(),
                &[
                    OsStr::new("commit-tree"),
                    OsStr::new(&tree),
                    OsStr::new("-m"),
                    OsStr::new("x"),
                ],
                b"",
            );
            world.git(&["tag", "pinned", &commit]);
            // The directory exists in the work tree, as a plain directory.
            world.scratch.make_dirs("hub/layer").unwrap();

            let text = error_text(world.resolve("pinned"));

            assert!(
                text.starts_with("error [source] cannot resolve ref pinned for 'path:"),
                "{kind}: {text}"
            );
            assert_eq!(
                world.cache_names(),
                Vec::<String>::new(),
                "{kind}: nothing is written"
            );
        }
    }

    #[test]
    fn a_ref_the_layer_did_not_exist_at_and_a_directory_outside_a_repository_cannot_be_resolved() {
        let world = World::new();
        world.commit(&[("README.md", "no layer yet\n")]);
        world.git(&["tag", "early"]);
        world.commit(LAYER);
        let text = error_text(world.resolve("early"));
        assert!(
            text.starts_with("error [source] cannot resolve ref early for "),
            "{text}"
        );
        let text = error_text(world.resolve("no-such-ref"));
        assert!(text.contains("cannot resolve ref no-such-ref"), "{text}");
        // A folder that no repository holds.
        world.scratch.make_dirs("repo/plain").unwrap();
        let text = error_text(world.resolve_in(&world.path("repo"), "plain", "main", None));
        assert!(text.contains("cannot resolve ref main"), "{text}");
    }

    #[test]
    fn a_repository_top_at_the_home_folder_or_in_an_account_folder_is_refused_before_git_reads_it()
    {
        for top in ["home", "home/.claude-work"] {
            let world = World::new();
            world
                .scratch
                .make_dirs(format!("{top}/layer"))
                .expect("made");
            world.put(
                &world.path(&format!("{top}/layer/layer.yaml")),
                "name: team\n",
            );
            let repository = world.path(top);
            for args in [
                vec!["init", "-q", "--initial-branch=main"],
                vec!["add", "-A"],
                vec![
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.com",
                    "commit",
                    "-q",
                    "-m",
                    "c",
                ],
            ] {
                let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
                world.git_in(&repository, &args, b"");
            }
            let diagnostic = world
                .resolve_in(&repository, "layer", "main", None)
                .expect_err("refused");
            assert_eq!(
                diagnostic.to_string(),
                format!(
                    "error [source] layer 'path:{}/layer' touches an account folder or the home folder; alz never reads there",
                    repository.display()
                ),
                "{top}"
            );
            assert!(!world.scratch.exists("xdg/agent-layerz/tree2@"), "{top}");
            assert_eq!(world.cache_names(), Vec::<String>::new(), "{top}");
        }
    }

    #[test]
    fn a_layer_at_the_top_of_its_repository_is_the_commit_s_own_tree() {
        let world = World::new();
        let commit = world.commit(&[("layer.yaml", "name: team\n"), ("a.md", "hello\n")]);
        let tree = id(&world.git(&["rev-parse", &format!("{commit}^{{tree}}")]));
        let (runner, location, cache) = world.parts(true);
        let source = Source::try_from(format!("path:{}", world.hub().display()).as_str()).unwrap();
        let git_ref = GitRef::try_from("main").unwrap();
        let read = PinnedTreeReader::new(&cache, &location, &runner)
            .resolve(&PinnedLayer {
                source: &source,
                git_ref: &git_ref,
                real_dir: &absolute(&world.hub()),
                folders: &world.folders(),
                locked: None,
            })
            .expect("resolved");
        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        assert_eq!(world.text(&world.entry(tree).join("a.md")), "hello\n");
    }

    #[test]
    fn a_repository_that_lacks_the_blobs_is_an_error_and_writes_nothing() {
        let world = World::new();
        let version = world.git(&["--version"]);
        let minor: u32 = version
            .split('.')
            .nth(1)
            .and_then(|part| part.parse().ok())
            .unwrap_or(0);
        if minor < 44 {
            eprintln!("skipped: GIT_NO_LAZY_FETCH needs git 2.44 ({version})");
            return;
        }
        world.commit(LAYER);
        world.git(&["config", "uploadpack.allowFilter", "true"]);
        let hub = world.hub();
        world.git_in(
            &world.root,
            &[
                OsStr::new("-c"),
                OsStr::new("protocol.file.allow=always"),
                OsStr::new("clone"),
                OsStr::new("-q"),
                OsStr::new("--filter=blob:none"),
                OsStr::new("--no-checkout"),
                OsStr::new(&format!("file://{}", hub.display())),
                OsStr::new("partial"),
            ],
            b"",
        );
        world.scratch.make_dirs("partial/layer").unwrap();

        let text = error_text(world.resolve_in(&world.path("partial"), "layer", "main", None));

        assert!(
            text.starts_with("error [source] cannot read 'path:"),
            "{text}"
        );
        assert!(
            text.contains(" at main: the repository does not hold the file '"),
            "{text}"
        );
        assert_eq!(world.cache_names(), Vec::<String>::new());
    }

    // --- AC #2: paths that are refused before anything is written --------------------------------

    fn raw(blob: &str, name: &[u8]) -> Vec<u8> {
        [format!("100644 blob {blob}\t").as_bytes(), name, b"\0"].concat()
    }

    #[test]
    fn a_tree_path_that_is_not_utf8_is_the_layer_error_and_nothing_is_written() {
        let world = World::new();
        let blob = world.blob("x\n");
        world.commit_raw_layer(&[raw(&blob, b"ok.md"), raw(&blob, b"bad\xff.md")].concat());
        let text = error_text(world.resolve("main"));
        assert!(
            text.starts_with("error [layer] a file or folder name in layer 'path:"),
            "{text}"
        );
        assert!(text.ends_with("is not valid UTF-8; rename it"), "{text}");
        assert!(
            !text.contains('\u{fffd}') && !text.contains('\u{ff}'),
            "{text}"
        );
        assert_eq!(world.cache_names(), Vec::<String>::new());
    }

    #[test]
    fn a_tree_path_with_a_dot_segment_a_backslash_or_a_git_name_is_refused_before_a_write() {
        for name in ["..", ".GIT", ".git.", "a\\b.md"] {
            let world = World::new();
            let blob = world.blob("x\n");
            world.commit_raw_layer(&[raw(&blob, b"ok.md"), raw(&blob, name.as_bytes())].concat());
            let text = error_text(world.resolve("main"));
            assert!(
                text.starts_with("error [layer] the git tree of layer 'path:")
                    && text.contains("backslash, a '.', '..' or '.git' segment"),
                "{name}: {text}"
            );
            assert_eq!(world.cache_names(), Vec::<String>::new(), "{name}");
        }
    }

    #[test]
    fn two_names_equal_after_nfc_and_lowercasing_are_refused_before_a_write() {
        for pair in [("a.md", "A.md"), ("\u{e9}.md", "e\u{301}.md")] {
            let world = World::new();
            let blob = world.blob("x\n");
            world.commit_raw_layer(
                &[raw(&blob, pair.0.as_bytes()), raw(&blob, pair.1.as_bytes())].concat(),
            );
            let text = error_text(world.resolve("main"));
            assert!(
                text.contains("one file system would merge"),
                "{pair:?}: {text}"
            );
            assert_eq!(world.cache_names(), Vec::<String>::new(), "{pair:?}");
        }
    }

    #[test]
    fn a_name_that_is_not_in_nfc_is_written_and_verified_in_nfc() {
        let world = World::new();
        let blob = world.blob("x\n");
        let commit = world.commit_raw_layer(&raw(&blob, "e\u{301}.md".as_bytes()));
        let tree = world.tree_of(commit, "layer");
        world.resolve("main").expect("written");
        let again = world.resolve("main").expect("verified again");
        assert_eq!(again.layer.entry, EntryName::Tree(tree));
        assert_eq!(again.layer.read, ReadHow::Cached);
    }

    // --- blob ids --------------------------------------------------------------------------------

    #[test]
    fn the_blob_id_alz_computes_is_the_one_git_hash_object_computes() {
        let world = World::new();
        world.git(&["init", "-q"]);
        for content in [
            &b""[..],
            b"hello\n",
            b"crlf\r\nline\r\n",
            b"\x00\xff binary\n",
        ] {
            let from_git = world.git_in(
                &world.hub(),
                &[
                    OsStr::new("hash-object"),
                    OsStr::new("--no-filters"),
                    OsStr::new("--stdin"),
                ],
                content,
            );
            assert_eq!(BlobId::of(ObjectFormat::Sha1, content).id(), id(&from_git));
        }
    }

    #[test]
    fn a_sha256_repository_is_written_and_verified_with_sha256_blob_ids() {
        let world = World::new();
        let probe = world
            .command(&world.hub())
            .args(["init", "-q", "--object-format=sha256"])
            .output()
            .expect("git runs");
        if !probe.status.success() {
            eprintln!("skipped: this git has no sha256 repositories");
            return;
        }
        let commit = world.commit(LAYER);
        assert!(matches!(commit, ObjectId::Sha256(_)));
        let tree = world.tree_of(commit, "layer");
        let read = world.resolve("main").expect("resolved");
        assert_eq!(read.layer.entry, EntryName::Tree(tree));
        assert_eq!(world.text(&world.entry(tree).join("a.md")), "hello\n");
        world.resolve("main").expect("verified");
    }
}
