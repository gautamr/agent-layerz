//! Reading a stack's account declaration from the layer cache, without git (spec §7.13 "No fetch
//! on resolve", §5.3 "Cache integrity"; TASK-55.7.8). Every test builds its own machine in a
//! scratch folder: a `home`, the `repo`, the layers in `other`, and the cache under `home`. Nothing
//! reads the real `HOME`, `XDG_CACHE_HOME` or `~/.cache`.

#[path = "support/account_machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use alz_core::account::{AccountFolders, CachedDeclaration, Ignored, Miss, MissReason};
    use alz_core::cache::{CacheLocation, CloneKey, DirKey, EntryName};
    use alz_core::layer::{LayerName, Visibility};
    use alz_core::lint::parse_document;
    use alz_core::lock::{ContentHash, LayerFiles, Lock, LockedLayer, ObjectId};
    use alz_core::merge::Reason;
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::stack::{GitRef, Manifest, Source};
    use alz_io::{CacheTrust, CacheView, CachedDeclarationReader, ReadOnlyDir};
    use alz_test_support::{read_tree, running_as_root};

    use super::machine::Machine;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const PERSONAL: &str = "name: personal\nkind: personal\nvisibility: private\n";
    const CLIENT: &str = "name: acme\nkind: client\nvisibility: private\naccount: acme\n";
    const GITHUB: &str = "https://github.com/o/r.git";

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).expect("an id")
    }

    fn hash_of(files: &[(&str, &str)]) -> ContentHash {
        ContentHash::of(&LayerFiles::from_contents(files.iter().map(
            |(path, text)| {
                (
                    SafeRelPath::try_from(*path).expect("a path"),
                    text.as_bytes(),
                )
            },
        )))
    }

    fn locked(
        source: &str,
        rev: Option<&str>,
        sha: Option<&str>,
        files: &[(&str, &str)],
    ) -> LockedLayer {
        LockedLayer {
            name: LayerName::try_from("x").expect("a name"),
            source: Source::try_from(source).expect("a source"),
            rev: rev.map(|text| GitRef::try_from(text).expect("a ref")),
            sha: sha.map(id),
            content_hash: hash_of(files),
        }
    }

    fn lock_text(layers: Vec<LockedLayer>) -> String {
        Lock {
            visibility: Visibility::Private,
            layers,
            files: BTreeMap::new(),
        }
        .emit()
    }

    fn folders(root: &Path) -> AccountFolders {
        AccountFolders::from_env(&absolute(&root.join("home")), None, None)
    }

    fn absolute(path: &Path) -> AbsolutePath {
        AbsolutePath::try_from(path.to_str().expect("text")).expect("absolute")
    }

    /// Reads the declaration of the repository in `root`, with the cache at `home/.cache` or at
    /// `xdg` when given, and `mise_active` for `__MISE_DIFF`.
    fn read_with(root: &Path, xdg: Option<&str>, mise_active: bool) -> CachedDeclaration {
        let home = root.join("home");
        let folders = folders(root);
        let repo = absolute(&root.join("repo"));
        let xdg = xdg.map(|relative| root.join(relative).to_str().expect("text").to_owned());
        let location = CacheLocation::from_env(home.to_str(), xdg.as_deref()).expect("a location");
        let default = CacheLocation::from_env(home.to_str(), None).ok();
        let trust = CacheTrust::new(location, default, mise_active);
        let view = match CacheView::open(&trust, &folders, &repo) {
            Ok(view) => view,
            Err(reason) => return CachedDeclaration::Miss(Miss::of(reason)),
        };
        let dir = ReadOnlyDir::open(&root.join("repo")).expect("the repository");
        let reader = match CachedDeclarationReader::new(&dir, &repo, &folders, &view) {
            Ok(reader) => reader,
            Err(reason) => return CachedDeclaration::Miss(Miss::of(reason)),
        };
        let text = fs::read(root.join("repo/layer.yaml")).expect("a manifest");
        let node = parse_document(&text).expect("valid YAML");
        let manifest = Manifest::from_yaml(&node).expect("a manifest");
        reader.read(&manifest)
    }

    fn read(root: &Path) -> CachedDeclaration {
        read_with(root, None, false)
    }

    fn miss(result: &CachedDeclaration) -> Option<MissReason> {
        match result {
            CachedDeclaration::Miss(found) => Some(found.reason()),
            CachedDeclaration::None { .. }
            | CachedDeclaration::Declared { .. }
            | CachedDeclaration::Conflict { .. } => None,
        }
    }

    /// A repository whose stack is the personal layer in place and a client `github:` layer from
    /// the cache.
    struct World {
        machine: Machine,
        root: PathBuf,
    }

    impl World {
        fn new() -> Self {
            let machine = Machine::new();
            let root = machine.path("");
            machine.make_dirs("home/.cache/agent-layerz");
            machine.make_dirs("other/personal");
            machine.write("other/personal/layer.yaml", PERSONAL);
            Self { machine, root }
        }

        fn manifest(&self, stack: &str) {
            self.machine
                .write("repo/layer.yaml", &format!("stack:\n{stack}"));
        }

        fn lock(&self, layers: Vec<LockedLayer>) {
            self.machine.write("repo/layer.lock", &lock_text(layers));
        }

        fn personal() -> LockedLayer {
            locked(
                "path:../other/personal",
                None,
                None,
                &[("layer.yaml", PERSONAL)],
            )
        }

        fn cache(name: &EntryName) -> String {
            format!("home/.cache/agent-layerz/{name}")
        }

        /// The github layer with `files` cached as the commit `SHA`, and the lock that pins it.
        fn github(&self, files: &[(&str, &str)]) {
            self.manifest("  - source: path:../other/personal\n  - source: github:o/r@v1\n");
            self.lock(vec![
                Self::personal(),
                locked(
                    "github:o/r@v1",
                    Some("0123456789ab"),
                    Some(SHA),
                    &[("layer.yaml", CLIENT)],
                ),
            ]);
            let entry = Self::cache(&EntryName::Git {
                key: CloneKey::of(GITHUB),
                commit: id(SHA),
            });
            self.machine.make_dirs(&entry);
            for (path, text) in files {
                self.machine.write(&format!("{entry}/{path}"), text);
            }
        }

        fn github_entry() -> String {
            Self::cache(&EntryName::Git {
                key: CloneKey::of(GITHUB),
                commit: id(SHA),
            })
        }

        /// The client layer in `other/client`, pinned to `v1`: its `tree2@` entry and its index.
        fn pinned(&self, index: &str) {
            self.manifest(
                "  - source: path:../other/personal\n  - source: path:../other/client\n    ref: v1\n",
            );
            self.lock(vec![
                Self::personal(),
                locked(
                    "path:../other/client",
                    Some("v1"),
                    Some(SHA),
                    &[("layer.yaml", CLIENT)],
                ),
            ]);
            self.machine.make_dirs("other/client");
            let tree = Self::cache(&EntryName::Tree(id(TREE)));
            self.machine.make_dirs(&tree);
            self.machine.write(&format!("{tree}/layer.yaml"), CLIENT);
            let key = DirKey::of(&absolute(&self.machine.path("other/client")));
            let name = EntryName::PathIndex {
                dir: key,
                commit: id(SHA),
            };
            self.machine.write(&Self::cache(&name), index);
        }

        fn index_name(&self) -> String {
            let key = DirKey::of(&absolute(&self.machine.path("other/client")));
            Self::cache(&EntryName::PathIndex {
                dir: key,
                commit: id(SHA),
            })
        }

        fn result(&self) -> CachedDeclaration {
            read(&self.root)
        }

        fn miss(&self) -> Option<MissReason> {
            miss(&self.result())
        }
    }

    fn set_mode(world: &World, relative: &str, mode: u32) {
        fs::set_permissions(
            world.machine.path(relative),
            fs::Permissions::from_mode(mode),
        )
        .expect("changed");
    }

    fn declared_layer(result: &CachedDeclaration) -> Option<String> {
        match result {
            CachedDeclaration::Declared { declaration, .. } => Some(declaration.layer.to_string()),
            CachedDeclaration::Miss(_)
            | CachedDeclaration::None { .. }
            | CachedDeclaration::Conflict { .. } => None,
        }
    }

    #[test]
    fn a_github_layer_is_read_from_the_entry_of_the_commit_the_lock_pins() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let result = world.result();
        let CachedDeclaration::Declared { declaration, .. } = result else {
            panic!("expected a declaration: {result:?}");
        };
        assert_eq!(declaration.layer.as_str(), "acme");
        assert_eq!(declaration.position, 1);
        assert_eq!(declaration.revision, Some(id(SHA)));
        assert_eq!(declaration.recorded_source, GITHUB);
    }

    #[test]
    fn an_unpinned_path_layer_is_read_in_place_and_an_edit_since_the_build_is_a_content_hash_miss()
    {
        let world = World::new();
        world.machine.write(
            "other/personal/layer.yaml",
            &format!("{PERSONAL}account: work\n"),
        );
        world.manifest("  - source: path:../other/personal\n");
        world.lock(vec![locked(
            "path:../other/personal",
            None,
            None,
            &[("layer.yaml", &format!("{PERSONAL}account: work\n"))],
        )]);
        let result = world.result();
        assert_eq!(declared_layer(&result).as_deref(), Some("personal"));
        let CachedDeclaration::Declared { declaration, .. } = result else {
            panic!("expected a declaration");
        };
        assert_eq!(
            declaration.recorded_source,
            world.machine.path("other/personal").to_str().expect("text")
        );
        world
            .machine
            .write("other/personal/NOTES.md", "added after the build\n");
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
    }

    #[test]
    fn a_pinned_path_layer_is_read_through_its_index_entry() {
        let world = World::new();
        world.pinned(TREE);
        assert_eq!(declared_layer(&world.result()).as_deref(), Some("acme"));
    }

    #[test]
    fn each_failure_of_the_index_entry_is_its_own_miss() {
        let world = World::new();
        world.pinned(TREE);
        let index = world.index_name();
        fs::remove_file(world.machine.path(&index)).expect("removed");
        assert_eq!(world.miss(), Some(MissReason::IndexMissing));
        for content in ["not an id", &format!("{TREE}\n"), &"a".repeat(64), ""] {
            world.machine.write(&index, content);
            assert_eq!(world.miss(), Some(MissReason::IndexInvalid), "{content:?}");
        }
        fs::remove_file(world.machine.path(&index)).expect("removed");
        world.machine.write("other/good-index", TREE);
        world.machine.link(
            &index,
            world
                .machine
                .path("other/good-index")
                .to_str()
                .expect("text"),
        );
        assert_eq!(world.miss(), Some(MissReason::IndexInvalid));
    }

    #[test]
    fn a_pinned_layer_whose_tree_entry_is_gone_is_a_miss_though_its_index_is_there() {
        let world = World::new();
        world.pinned(TREE);
        fs::remove_dir_all(
            world
                .machine
                .path(&World::cache(&EntryName::Tree(id(TREE)))),
        )
        .expect("removed");
        assert_eq!(world.miss(), Some(MissReason::CacheEntryMissing));
    }

    #[test]
    fn a_lock_that_records_another_ref_than_the_manifest_is_a_miss_until_the_next_build() {
        let world = World::new();
        world.pinned(TREE);
        world.lock(vec![
            World::personal(),
            locked(
                "path:../other/client",
                Some("v0"),
                Some(SHA),
                &[("layer.yaml", CLIENT)],
            ),
        ]);
        assert_eq!(world.miss(), Some(MissReason::LockRefMismatch));
    }

    #[test]
    fn the_lock_is_checked_before_a_layer_is_read() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let good = fs::read_to_string(world.machine.path("repo/layer.lock")).expect("a lock");
        fs::remove_file(world.machine.path("repo/layer.lock")).expect("removed");
        assert_eq!(world.miss(), Some(MissReason::LockMissing));
        world.machine.write("other/good.lock", &good);
        world.machine.link(
            "repo/layer.lock",
            world
                .machine
                .path("other/good.lock")
                .to_str()
                .expect("text"),
        );
        assert_eq!(world.miss(), Some(MissReason::LockUnreadable));
        fs::remove_file(world.machine.path("repo/layer.lock")).expect("removed");
        world.machine.write(
            "repo/layer.lock",
            &good.replace("\"contentHash\"", "\"contentHaSh\""),
        );
        assert_eq!(world.miss(), Some(MissReason::LockUnreadable));
        world
            .machine
            .write("repo/layer.lock", &good.replace(SHA, "not-a-commit"));
        assert_eq!(world.miss(), Some(MissReason::LockUnreadable));
        world
            .machine
            .write("repo/layer.lock", &lock_text(vec![World::personal()]));
        assert_eq!(world.miss(), Some(MissReason::LockEntryMissing));
        world.machine.write(
            "repo/layer.lock",
            &lock_text(vec![World::personal(), World::personal()]),
        );
        assert_eq!(world.miss(), Some(MissReason::LockEntryDuplicate));
    }

    #[test]
    fn a_git_layer_with_no_entry_in_the_cache_or_a_link_in_its_place_is_a_miss() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let entry = World::github_entry();
        fs::rename(world.machine.path(&entry), world.machine.path("other/copy")).expect("moved");
        assert_eq!(world.miss(), Some(MissReason::CacheEntryMissing));
        world.machine.link(
            &entry,
            world.machine.path("other/copy").to_str().expect("text"),
        );
        assert_eq!(world.miss(), Some(MissReason::CacheEntryNotDirectory));
    }

    #[test]
    fn a_cached_file_that_was_edited_or_that_the_hash_does_not_cover_is_a_content_hash_miss() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let entry = World::github_entry();
        world.machine.write(
            &format!("{entry}/layer.yaml"),
            &format!("{CLIENT}# edited\n"),
        );
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
        world.machine.write(&format!("{entry}/layer.yaml"), CLIENT);
        assert_eq!(declared_layer(&world.result()).as_deref(), Some("acme"));
        world
            .machine
            .write(&format!("{entry}/extra.md"), "not in the lock\n");
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
    }

    #[test]
    fn an_invalid_layer_yaml_is_parsed_only_after_the_content_check() {
        let world = World::new();
        let broken = "name: [unclosed\n";
        world.github(&[("layer.yaml", broken)]);
        // The lock covers other bytes, so the check fails before the broken file is parsed.
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
        world.lock(vec![
            World::personal(),
            locked(
                "github:o/r@v1",
                Some("0123456789ab"),
                Some(SHA),
                &[("layer.yaml", broken)],
            ),
        ]);
        assert_eq!(world.miss(), Some(MissReason::LayerInvalid));
    }

    #[test]
    fn a_layer_with_a_env_file_is_invalid_and_the_file_is_not_opened() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT), (".env", "TOKEN=1\n")]);
        let entry = World::github_entry();
        assert_eq!(world.miss(), Some(MissReason::LayerInvalid));
        if !running_as_root() {
            let path = world.machine.path(&format!("{entry}/.env"));
            fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o000))
                .expect("locked");
            assert_eq!(world.miss(), Some(MissReason::LayerInvalid));
        }
    }

    /// §5.3 ".env files": account resolution counts a layer that holds a `.env` file as a
    /// `layer-invalid` miss only after the content check, so a layer whose other files differ from
    /// the lock is the content-hash miss, with or without the `.env` file. The `.env` file is not
    /// hashed, so adding one to a layer that matches the lock changes nothing but the verdict.
    #[test]
    fn the_env_check_comes_after_the_content_hash_check() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT), (".env", "TOKEN=1\n")]);
        let entry = World::github_entry();
        assert_eq!(world.miss(), Some(MissReason::LayerInvalid));
        world.machine.write(
            &format!("{entry}/layer.yaml"),
            &format!("{CLIENT}# edited\n"),
        );
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
        world.machine.write(&format!("{entry}/layer.yaml"), CLIENT);
        fs::remove_file(world.machine.path(&format!("{entry}/.env"))).expect("removed");
        assert_eq!(declared_layer(&world.result()).as_deref(), Some("acme"));
    }

    #[test]
    fn a_link_inside_a_cached_layer_is_never_followed() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let entry = World::github_entry();
        world.machine.write("other/good-copy", CLIENT);
        fs::remove_file(world.machine.path(&format!("{entry}/layer.yaml"))).expect("removed");
        world.machine.link(
            &format!("{entry}/layer.yaml"),
            world
                .machine
                .path("other/good-copy")
                .to_str()
                .expect("text"),
        );
        assert_eq!(world.miss(), Some(MissReason::ContentHash));
    }

    #[test]
    fn a_file_name_that_is_not_valid_utf8_makes_the_layer_invalid() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let entry = World::github_entry();
        let name = OsStr::from_bytes(b"bad-\xff.md");
        // Some file systems (APFS) refuse such a name; there the layer cannot exist.
        if fs::write(world.machine.path(&entry).join(name), "x").is_err() {
            return;
        }
        assert_eq!(world.miss(), Some(MissReason::LayerInvalid));
    }

    #[test]
    fn a_layer_of_more_than_2000_entries_or_16_mib_is_unreadable() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let entry = World::github_entry();
        world.machine.make_dirs(&format!("{entry}/many"));
        for number in 0..2000 {
            world.machine.write(&format!("{entry}/many/f{number}"), "");
        }
        assert_eq!(world.miss(), Some(MissReason::Unreadable));
        fs::remove_dir_all(world.machine.path(&format!("{entry}/many"))).expect("removed");
        let big = "a".repeat(9 * 1024 * 1024);
        world.machine.write(&format!("{entry}/one.bin"), &big);
        world.machine.write(&format!("{entry}/two.bin"), &big);
        assert_eq!(world.miss(), Some(MissReason::Unreadable));
    }

    #[test]
    fn a_cache_that_touches_an_account_folder_or_the_repository_is_a_miss() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        world.machine.make_dirs("home/.claude-work/cache");
        let moved = read_with(&world.root, Some("home/.claude-work/cache"), false);
        assert_eq!(miss(&moved), Some(MissReason::AccountFolder));
        world.machine.make_dirs("repo/cache");
        let inside = read_with(&world.root, Some("repo/cache"), false);
        assert_eq!(miss(&inside), Some(MissReason::CacheInRepo));
    }

    #[test]
    fn a_cache_moved_away_from_the_default_while_mise_is_active_is_not_trusted() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        world.machine.make_dirs("other/cache");
        let moved = read_with(&world.root, Some("other/cache"), true);
        assert_eq!(miss(&moved), Some(MissReason::CacheLocation));
        let calm = read_with(&world.root, Some("other/cache"), false);
        assert_eq!(miss(&calm), Some(MissReason::CacheEntryMissing));
        let same = read_with(&world.root, Some("home/.cache"), true);
        assert_eq!(declared_layer(&same).as_deref(), Some("acme"));
    }

    #[test]
    fn a_cache_root_that_group_or_others_can_write_is_not_read() {
        if running_as_root() {
            return;
        }
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        assert_eq!(declared_layer(&world.result()).as_deref(), Some("acme"));
        for mode in [0o775, 0o757] {
            set_mode(&world, "home/.cache/agent-layerz", mode);
            assert_eq!(world.miss(), Some(MissReason::Unreadable), "{mode:o}");
        }
        set_mode(&world, "home/.cache/agent-layerz", 0o755);
        assert_eq!(declared_layer(&world.result()).as_deref(), Some("acme"));
    }

    #[test]
    fn a_cache_that_does_not_exist_yet_is_read_and_creates_nothing() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        fs::remove_dir_all(world.machine.path("home/.cache")).expect("removed");
        assert_eq!(world.miss(), Some(MissReason::CacheEntryMissing));
        assert!(!world.machine.path("home/.cache").exists());
    }

    #[test]
    fn sources_that_cannot_be_located_are_misses() {
        let world = World::new();
        world.manifest(
            "  - source: path:../other/personal\n  - source: git:https://h/a/../b.git@v1\n",
        );
        world.lock(vec![World::personal()]);
        assert_eq!(world.miss(), Some(MissReason::SourceInvalid));
        world.manifest("  - source: path:../other/nowhere\n");
        assert_eq!(world.miss(), Some(MissReason::LayerDirectoryMissing));
        world.machine.make_dirs("home/.claude-x/layer");
        world.manifest("  - source: path:../home/.claude-x/layer\n");
        assert_eq!(world.miss(), Some(MissReason::AccountFolder));
    }

    #[test]
    fn a_layer_inside_the_repository_is_reported_and_never_decides_or_blocks() {
        let world = World::new();
        world.machine.make_dirs("repo/.layer");
        world.machine.write(
            "repo/.layer/layer.yaml",
            &format!("{PERSONAL}account: work\n"),
        );
        world.manifest("  - source: path:./.layer\n");
        world.lock(vec![]);
        let result = world.result();
        let CachedDeclaration::None { ignored } = result else {
            panic!("expected no declaration: {result:?}");
        };
        let [
            Ignored {
                reasons,
                position: 0,
                ..
            },
        ] = ignored.as_slice()
        else {
            panic!("expected one ignored declaration: {ignored:?}");
        };
        assert!(reasons.contains(&Reason::InsideRepository));
        // A layer.yaml that cannot be read or parsed is skipped, with no miss.
        world
            .machine
            .write("repo/.layer/layer.yaml", "name: [broken\n");
        assert!(
            matches!(world.result(), CachedDeclaration::None { ignored } if ignored.is_empty())
        );
    }

    #[test]
    fn a_repository_inside_an_account_folder_is_refused_before_anything_is_read() {
        let world = World::new();
        world.machine.make_dirs("home/.claude-work/repo");
        let root = world.machine.path("home/.claude-work/repo");
        let dir = ReadOnlyDir::open(&root).expect("a directory");
        let folders = folders(&world.root);
        let trust = CacheTrust::new(
            CacheLocation::from_env(world.machine.path("home").to_str(), None).expect("a location"),
            None,
            false,
        );
        let view = CacheView::open(&trust, &folders, &absolute(&root)).expect("a view");
        let real = world.machine.abs("home/.claude-work/repo");
        let refused = CachedDeclarationReader::new(&dir, &real, &folders, &view);
        assert!(matches!(refused, Err(MissReason::AccountFolder)));
    }

    #[test]
    fn reading_leaves_the_cache_the_repository_and_the_layers_byte_identical() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        for scenario in 0..3 {
            match scenario {
                1 => world
                    .machine
                    .write(&format!("{}/extra.md", World::github_entry()), "x"),
                2 => fs::remove_file(world.machine.path("repo/layer.lock")).expect("removed"),
                _ => {}
            }
            let before = read_tree(&world.root).expect("a tree");
            let _ = world.result();
            assert_eq!(
                read_tree(&world.root).expect("a tree"),
                before,
                "scenario {scenario}"
            );
        }
    }

    /// The child of [`reading_starts_no_process`]: it reads with a `git` stand-in first on `PATH`.
    #[test]
    fn child_reads_with_a_git_stand_in_on_the_path() {
        // The parent starts this test with the machine as the working directory.
        if let Ok(root) = std::env::current_dir()
            && root.join("repo/layer.lock").is_file()
        {
            assert_eq!(declared_layer(&read(&root)).as_deref(), Some("acme"));
        }
    }

    #[test]
    fn reading_starts_no_process() {
        let world = World::new();
        world.github(&[("layer.yaml", CLIENT)]);
        let bin = world.machine.path("other/bin");
        fs::create_dir_all(&bin).expect("made");
        let log = world.machine.path("other/git-calls");
        let script = format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", log.display());
        fs::write(bin.join("git"), script).expect("written");
        fs::set_permissions(
            bin.join("git"),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .expect("executable");
        let status = Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "--exact",
                "tests::child_reads_with_a_git_stand_in_on_the_path",
            ])
            .env("PATH", &bin)
            .current_dir(&world.root)
            .env("HOME", world.machine.path("home"))
            .env_remove("XDG_CACHE_HOME")
            .status()
            .expect("the child runs");
        assert!(status.success());
        assert!(!log.exists(), "the stand-in git was started");
    }
}
