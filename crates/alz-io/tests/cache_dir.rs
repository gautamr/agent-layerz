//! The layer cache capability (spec §6 "The layer cache", TASK-52 AC #2, TASK-53; decision-27).

#[path = "support/machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use alz_core::account::AccountFolders;
    use alz_core::cache::{CacheLocation, DirKey, EntryName};
    use alz_core::lock::ObjectId;
    use alz_io::{CacheDir, CacheError, IndexError, Published, RepositoryPaths, TempEntry};

    use super::machine::Machine;

    fn id(digit: char) -> ObjectId {
        ObjectId::try_from(digit.to_string().repeat(40).as_str()).expect("an object id")
    }

    fn mode(machine: &Machine, relative: &str) -> u32 {
        fs::metadata(machine.path(relative))
            .expect("exists")
            .permissions()
            .mode()
            & 0o777
    }

    fn set_mode(machine: &Machine, relative: &str, mode: u32) {
        fs::set_permissions(machine.path(relative), fs::Permissions::from_mode(mode)).expect("set");
    }

    fn location(machine: &Machine, xdg: Option<&str>) -> CacheLocation {
        let xdg = xdg.map(|relative| machine.path(relative).to_str().expect("text").to_owned());
        CacheLocation::from_env(machine.path("home").to_str(), xdg.as_deref()).expect("a location")
    }

    fn open_with(
        machine: &Machine,
        xdg: Option<&str>,
        folders: &AccountFolders,
    ) -> Result<CacheDir, CacheError> {
        CacheDir::open_or_create(
            &location(machine, xdg),
            folders,
            &RepositoryPaths::new(machine.abs("repo"), machine.abs("repo")),
        )
    }

    fn open(machine: &Machine, xdg: Option<&str>) -> Result<CacheDir, CacheError> {
        open_with(machine, xdg, &machine.folders(None, None))
    }

    fn tree(digit: char) -> EntryName {
        EntryName::Tree(id(digit))
    }

    fn listing(machine: &Machine, relative: &str) -> Vec<String> {
        let mut names = fs::read_dir(machine.path(relative))
            .expect("listed")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn a_cache_root_inside_an_account_folder_is_refused_before_anything_in_it_is_looked_at() {
        let machine = Machine::new();
        machine.make_dirs("home/.claude-x");
        machine.make_dirs("other/cfg");
        machine.link("other/link", "../home/.claude-x");
        let folders = machine.folders(Some("other/cfg"), None);
        let locked = machine.lock_down("home/.claude-x");
        for xdg in [
            "home/.claude-x/cache",
            "home/.claude-x",
            "other/link",
            "other/link/cache",
            "other/cfg/cache",
        ] {
            let result = open_with(&machine, Some(xdg), &folders);
            assert!(
                matches!(result, Err(CacheError::TouchesAccountFolder)),
                "{xdg}: {result:?}"
            );
        }
        drop(locked);
        assert_eq!(listing(&machine, "home/.claude-x"), Vec::<String>::new());
        assert_eq!(listing(&machine, "other/cfg"), Vec::<String>::new());
    }

    #[test]
    fn a_refused_cache_root_creates_no_folder() {
        let machine = Machine::new();
        let result = open(&machine, Some("home/.claude-new/deep/cache"));
        assert!(matches!(result, Err(CacheError::TouchesAccountFolder)));
        assert_eq!(listing(&machine, "home"), Vec::<String>::new());
    }

    #[test]
    fn a_cache_root_that_is_the_home_folder_or_above_it_is_refused_and_nothing_is_created() {
        let machine = Machine::new();
        // HOME is `<root>/agent-layerz/home`, so the cache root `<root>/agent-layerz` is above it.
        let folders = AccountFolders::from_env(&machine.abs("agent-layerz/home"), None, None);
        let result = open_with(&machine, Some("."), &folders);
        assert!(
            matches!(result, Err(CacheError::TouchesAccountFolder)),
            "{result:?}"
        );
        assert_eq!(listing(&machine, ""), ["home", "other", "repo"]);
    }

    #[test]
    fn missing_folders_of_the_default_root_are_created_with_mode_0700_and_an_existing_one_keeps_its_mode()
     {
        let machine = Machine::new();
        machine.make_dirs("home/.cache");
        set_mode(&machine, "home/.cache", 0o755);
        let cache = open(&machine, None).expect("opened");
        assert_eq!(mode(&machine, "home/.cache"), 0o755);
        assert_eq!(mode(&machine, "home/.cache/agent-layerz"), 0o700);
        assert_eq!(
            cache.real_path().as_str(),
            machine
                .path("home/.cache/agent-layerz")
                .to_str()
                .expect("text")
        );

        let other = Machine::new();
        open(&other, Some("other/a/b")).expect("opened");
        for folder in ["other/a", "other/a/b", "other/a/b/agent-layerz"] {
            assert_eq!(mode(&other, folder), 0o700, "{folder}");
        }
    }

    #[test]
    fn an_empty_or_relative_xdg_cache_home_is_the_default_location() {
        let machine = Machine::new();
        for xdg in [Some(""), Some("cache"), None] {
            let location =
                CacheLocation::from_env(machine.path("home").to_str(), xdg).expect("a location");
            let cache = CacheDir::open_or_create(
                &location,
                &machine.folders(None, None),
                &RepositoryPaths::new(machine.abs("repo"), machine.abs("repo")),
            )
            .expect("opened");
            assert!(
                cache
                    .real_path()
                    .as_str()
                    .ends_with("/home/.cache/agent-layerz")
            );
        }
    }

    #[test]
    fn a_link_in_the_roots_own_path_is_followed_and_the_root_is_made_where_it_leads() {
        let machine = Machine::new();
        machine.make_dirs("other/real");
        machine.link("other/link", "real");
        let cache = open(&machine, Some("other/link/base")).expect("opened");
        assert_eq!(mode(&machine, "other/real/base/agent-layerz"), 0o700);
        assert_eq!(
            cache.real_path().as_str(),
            machine
                .path("other/real/base/agent-layerz")
                .to_str()
                .expect("text")
        );
    }

    #[test]
    fn an_absolute_xdg_cache_home_inside_the_repository_is_refused_as_written_and_as_a_real_path() {
        let machine = Machine::new();
        machine.make_dirs("repo/sub");
        machine.link("other/into-repo", "../repo/sub");
        for xdg in [
            "repo/cache",
            "repo",
            "repo/sub/deeper",
            "other/into-repo/cache",
        ] {
            let result = open(&machine, Some(xdg));
            assert!(
                matches!(result, Err(CacheError::InsideRepository)),
                "{xdg}: {result:?}"
            );
        }
        // Nothing was created in the repository, not even where the link led.
        assert_eq!(listing(&machine, "repo"), ["sub"]);
        assert_eq!(listing(&machine, "repo/sub"), Vec::<String>::new());
    }

    #[test]
    fn a_refusal_inside_the_repository_is_the_error_cache_with_the_spec_text() {
        let machine = Machine::new();
        let location = location(&machine, Some("repo/cache"));
        let error = CacheDir::open_or_create(
            &location,
            &machine.folders(None, None),
            &RepositoryPaths::new(machine.abs("repo"), machine.abs("repo")),
        )
        .expect_err("refused");
        assert_eq!(
            error.into_diagnostic(&location).to_string(),
            format!(
                "error [cache] the layer cache at {}/repo/cache/agent-layerz lies inside the repository; set XDG_CACHE_HOME to a folder outside it",
                machine
                    .path("")
                    .to_str()
                    .expect("text")
                    .trim_end_matches('/')
            )
        );
    }

    #[test]
    fn an_entry_that_is_a_link_or_not_a_folder_is_never_followed() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let root = "other/c/agent-layerz";
        machine.make_dirs("other/outside");
        machine.write("other/outside/layer.yaml", "name: outside\n");
        machine.make_dirs(&format!("{root}/{}", tree('1')));
        machine.link(&format!("{root}/{}", tree('2')), "../../outside");
        machine.write(&format!("{root}/{}", tree('3')), "a file");

        assert!(cache.entry(&tree('0')).expect("checked").is_none());
        assert!(cache.entry(&tree('1')).expect("checked").is_some());
        for digit in ['2', '3'] {
            assert!(matches!(
                cache.entry(&tree(digit)),
                Err(CacheError::EntryNotDirectory(name)) if name == tree(digit)
            ));
        }
    }

    #[test]
    fn a_temporary_entry_is_made_with_mode_0700_and_removed_with_its_content_on_drop() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        machine.make_dirs("other/outside");
        machine.write("other/outside/keep.txt", "keep");
        let name = {
            let temp = TempEntry::create(&cache).expect("made");
            let name = temp.name().to_owned();
            assert!(name.starts_with(".tmp-"));
            let path = format!("other/c/agent-layerz/{name}");
            assert_eq!(mode(&machine, &path), 0o700);
            machine.make_dirs(&format!("{path}/sub/deeper"));
            machine.write(&format!("{path}/sub/file"), "x");
            machine.link(&format!("{path}/sub/link"), "../../../../outside");
            machine.link(&format!("{path}/sub/deeper/dead"), "nowhere");
            name
        };
        assert!(
            !machine
                .path(&format!("other/c/agent-layerz/{name}"))
                .exists()
        );
        assert_eq!(listing(&machine, "other/outside"), ["keep.txt"]);
    }

    #[test]
    fn two_temporary_entries_have_different_names() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let first = TempEntry::create(&cache).expect("made");
        let second = TempEntry::create(&cache).expect("made");
        assert_ne!(first.name(), second.name());
    }

    #[test]
    fn publishing_moves_the_entry_and_never_replaces_one_that_is_there() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let root = "other/c/agent-layerz";
        let first = TempEntry::create(&cache).expect("made");
        machine.write(
            &format!("{root}/{}/layer.yaml", first.name()),
            "name: first\n",
        );
        let published = first.publish(&tree('a')).expect("published");
        assert!(matches!(published, Published::Created(_)));
        assert_eq!(listing(&machine, root), [tree('a').to_string()]);

        let second = TempEntry::create(&cache).expect("made");
        let second_name = second.name().to_owned();
        machine.write(
            &format!("{root}/{second_name}/layer.yaml"),
            "name: second\n",
        );
        match second.publish(&tree('a')).expect("published") {
            Published::Existing(dir) => {
                let path = alz_core::path::SafeRelPath::try_from("layer.yaml").expect("path");
                assert_eq!(dir.read_file(&path, 100).expect("read"), b"name: first\n");
            }
            Published::Created(_) => panic!("an existing entry was replaced"),
        }
        assert_eq!(
            fs::read_to_string(machine.path(&format!("{root}/{}/layer.yaml", tree('a'))))
                .expect("read"),
            "name: first\n"
        );
        // The entry that lost was removed.
        assert_eq!(listing(&machine, root), [tree('a').to_string()]);
    }

    #[test]
    fn publishing_onto_a_link_leaves_the_link_and_is_the_cache_error() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let root = "other/c/agent-layerz";
        machine.make_dirs("other/outside");
        machine.link(&format!("{root}/{}", tree('b')), "../../outside");
        let temp = TempEntry::create(&cache).expect("made");
        let result = temp.publish(&tree('b'));
        assert!(
            matches!(result, Err(CacheError::EntryNotDirectory(_))),
            "{result:?}"
        );
        assert_eq!(listing(&machine, "other/outside"), Vec::<String>::new());
        assert!(
            fs::symlink_metadata(machine.path(&format!("{root}/{}", tree('b'))))
                .expect("still there")
                .file_type()
                .is_symlink()
        );
        assert_eq!(listing(&machine, root), [tree('b').to_string()]);
    }

    fn dir_key(machine: &Machine) -> DirKey {
        DirKey::of(&machine.abs("repo"))
    }

    #[test]
    fn the_index_is_written_through_an_exclusive_temporary_file_and_a_rename() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let root = "other/c/agent-layerz";
        let key = dir_key(&machine);
        let name = EntryName::PathIndex {
            dir: key,
            commit: id('c'),
        }
        .to_string();

        assert_eq!(cache.read_index(key, id('c')), None);
        cache.write_index(key, id('c'), id('d')).expect("written");
        assert_eq!(cache.read_index(key, id('c')), Some(id('d')));
        assert_eq!(listing(&machine, root), std::slice::from_ref(&name));
        assert_eq!(mode(&machine, &format!("{root}/{name}")), 0o600);
        assert_eq!(
            fs::read_to_string(machine.path(&format!("{root}/{name}"))).expect("read"),
            "d".repeat(40)
        );

        cache.write_index(key, id('c'), id('e')).expect("rewritten");
        assert_eq!(cache.read_index(key, id('c')), Some(id('e')));
        assert_eq!(listing(&machine, root), [name]);
    }

    #[test]
    fn a_folder_at_the_index_name_is_the_cache_error_and_a_link_there_is_replaced_not_followed() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let root = "other/c/agent-layerz";
        let key = dir_key(&machine);
        let name = EntryName::PathIndex {
            dir: key,
            commit: id('c'),
        }
        .to_string();
        machine.make_dirs(&format!("{root}/{name}"));
        let blocked = cache.write_index(key, id('c'), id('d'));
        assert!(
            matches!(blocked, Err(IndexError::Blocked(_))),
            "{blocked:?}"
        );
        assert_eq!(cache.read_index(key, id('c')), None);
        assert_eq!(listing(&machine, root), std::slice::from_ref(&name));
        fs::remove_dir(machine.path(&format!("{root}/{name}"))).expect("removed");

        machine.write("other/victim", "untouched");
        machine.link(&format!("{root}/{name}"), "../../victim");
        assert_eq!(cache.read_index(key, id('c')), None);
        cache.write_index(key, id('c'), id('d')).expect("written");
        assert_eq!(
            fs::read_to_string(machine.path("other/victim")).expect("read"),
            "untouched"
        );
        assert_eq!(cache.read_index(key, id('c')), Some(id('d')));
    }

    #[test]
    fn only_a_regular_file_of_one_lowercase_object_id_is_an_index() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        let key = dir_key(&machine);
        let path = format!(
            "other/c/agent-layerz/{}",
            EntryName::PathIndex {
                dir: key,
                commit: id('c')
            }
        );
        for bad in [
            "",
            "dddd",
            &format!("{}\n", "d".repeat(40)),
            &"D".repeat(40),
            &"d".repeat(41),
            &"d".repeat(100),
        ] {
            machine.write(&path, bad);
            assert_eq!(cache.read_index(key, id('c')), None, "{bad:?}");
        }
        machine.write(&path, &"d".repeat(64));
        assert!(cache.read_index(key, id('c')).is_some());
    }

    #[test]
    fn a_cache_that_cannot_be_written_is_the_cache_index_warning() {
        let machine = Machine::new();
        let cache = open(&machine, Some("other/c")).expect("opened");
        set_mode(&machine, "other/c/agent-layerz", 0o500);
        let result = cache.write_index(dir_key(&machine), id('c'), id('d'));
        set_mode(&machine, "other/c/agent-layerz", 0o700);
        match result {
            Err(error @ IndexError::Unwritable(..)) => {
                let line = error
                    .into_diagnostic(&location(&machine, Some("other/c")))
                    .to_string();
                assert!(
                    line.starts_with("warning [cache-index] cache entry pathref@"),
                    "{line}"
                );
            }
            // Running as a user that ignores modes: nothing to show.
            Ok(()) => {}
            Err(other) => panic!("{other:?}"),
        }
    }
}
