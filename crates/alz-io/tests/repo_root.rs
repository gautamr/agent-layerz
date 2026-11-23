//! The repository writer (spec §4.3, TASK-55.6.7): what it refuses, what it writes and how, in a
//! temporary repository with a folder beside it that must never change.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    use alz_core::account::AccountFolders;
    use alz_core::layer::Visibility;
    use alz_core::lock::{FileHash, Lock};
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::render::gitignore::Generated;
    use alz_core::render::header::GENERATED_MARK;
    use alz_core::render::protected::Protected;
    use alz_core::render::tree::{FileMode, FileTree, GeneratedFile, OutputPath};
    use alz_core::render::write_plan::{Change, CollisionReason, WritePlan, plan_writes};
    use alz_io::{RepoError, RepoRoot};
    use alz_test_support::ScratchDir;
    use proptest::prelude::*;

    const MANAGED: [&str; 1] = [".claude/agents"];

    /// A scratch folder with `repo` (the repository), `outside` (what no write may reach) and
    /// `home`.
    struct Place {
        _scratch: ScratchDir,
        base: PathBuf,
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let base = fs::canonicalize(scratch.path()).expect("a real path");
            for name in ["repo", "outside", "home"] {
                fs::create_dir(base.join(name)).expect("made");
            }
            Self {
                _scratch: scratch,
                base,
            }
        }

        fn repo(&self) -> PathBuf {
            self.base.join("repo")
        }

        fn outside(&self) -> PathBuf {
            self.base.join("outside")
        }

        fn root(&self) -> RepoRoot {
            let text = |path: PathBuf| path.to_str().expect("text").to_owned();
            let absolute =
                |path: PathBuf| AbsolutePath::try_from(text(path).as_str()).expect("absolute");
            let folders = AccountFolders::from_env(&absolute(self.base.join("home")), None, None);
            RepoRoot::open(&absolute(self.repo()), &folders).expect("the repository opens")
        }

        fn write(&self, relative: &str, text: &str) {
            let path = self.repo().join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("made");
            }
            fs::write(path, text).expect("written");
        }

        fn read(&self, relative: &str) -> Option<String> {
            fs::read_to_string(self.repo().join(relative)).ok()
        }

        fn link(&self, relative: &str, target: &Path) {
            let path = self.repo().join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("made");
            }
            std::os::unix::fs::symlink(target, path).expect("linked");
        }
    }

    fn safe(path: &str) -> SafeRelPath {
        SafeRelPath::try_from(path).expect("a safe path")
    }

    fn tree(files: &[(&str, &str)]) -> FileTree {
        FileTree::from_files(files.iter().map(|(path, text)| {
            (
                OutputPath::try_from(*path).expect("a path"),
                GeneratedFile::text(text),
            )
        }))
        .expect("no collision")
    }

    fn lock_for(files: &FileTree) -> Lock {
        Lock {
            visibility: Visibility::Public,
            layers: Vec::new(),
            files: files
                .iter()
                .map(|(path, file)| (path.as_safe().clone(), FileHash::of(&file.bytes)))
                .collect(),
        }
    }

    fn previous_lock(files: &[(&str, &str)]) -> Lock {
        lock_for(&tree(files))
    }

    fn plan(root: &RepoRoot, files: &FileTree, previous: Option<&Lock>) -> WritePlan {
        let managed: Vec<SafeRelPath> = MANAGED.iter().map(|folder| safe(folder)).collect();
        let disk = root.observe(files, previous, &managed).expect("observed");
        plan_writes(
            files,
            &lock_for(files),
            previous,
            &disk,
            &Protected::new(&[], &[]),
            Generated::Committed,
        )
    }

    fn reasons(plan: &WritePlan) -> Vec<(String, CollisionReason)> {
        plan.collisions()
            .iter()
            .map(|collision| {
                (
                    collision.path().as_str().to_owned(),
                    collision.reason().clone(),
                )
            })
            .collect()
    }

    /// Every entry below `base`, links not followed: its kind and its bytes or target.
    fn snapshot(base: &Path) -> BTreeMap<String, String> {
        fn walk(base: &Path, folder: &Path, found: &mut BTreeMap<String, String>) {
            for entry in fs::read_dir(folder).expect("listed") {
                let path = entry.expect("an entry").path();
                let relative = path
                    .strip_prefix(base)
                    .expect("below")
                    .display()
                    .to_string();
                let metadata = fs::symlink_metadata(&path).expect("looked at");
                let kind = if metadata.is_symlink() {
                    format!("link {}", fs::read_link(&path).expect("read").display())
                } else if metadata.is_dir() {
                    walk(base, &path, found);
                    "folder".to_owned()
                } else if metadata.is_file() {
                    format!(
                        "file {:o} {}",
                        metadata.mode() & 0o777,
                        String::from_utf8_lossy(&fs::read(&path).expect("read"))
                    )
                } else {
                    "other".to_owned()
                };
                found.insert(relative, kind);
            }
        }
        let mut found = BTreeMap::new();
        walk(base, base, &mut found);
        found
    }

    #[test]
    fn a_link_at_a_generated_path_is_a_collision_and_its_target_is_never_touched() {
        let place = Place::new();
        fs::write(place.outside().join("target.txt"), "keep me").expect("written");
        place.link("AGENTS.md", &place.outside().join("target.txt"));
        // A lock that lists the path, with the hash of what the link leads to, changes nothing.
        let previous = previous_lock(&[("AGENTS.md", "keep me")]);
        let files = tree(&[("AGENTS.md", "generated")]);
        let before = snapshot(&place.base);
        let made = plan(&place.root(), &files, Some(&previous));
        assert_eq!(
            reasons(&made),
            [("AGENTS.md".to_owned(), CollisionReason::Link)]
        );
        assert!(made.into_clear().is_err());
        assert_eq!(snapshot(&place.base), before);
    }

    #[test]
    fn a_link_above_a_generated_path_is_a_collision_and_nothing_is_made_through_it() {
        let place = Place::new();
        place.link(".claude", &place.outside());
        let files = tree(&[(".claude/settings.json", "{}"), (".claude/rules/a.md", "x")]);
        let before = snapshot(&place.base);
        let made = plan(&place.root(), &files, None);
        assert_eq!(
            reasons(&made),
            [
                (".claude/rules/a.md".to_owned(), CollisionReason::Link),
                (".claude/settings.json".to_owned(), CollisionReason::Link),
            ]
        );
        assert_eq!(snapshot(&place.base), before);
        assert!(snapshot(&place.outside()).is_empty());
    }

    #[test]
    fn a_folder_that_becomes_a_link_after_the_plan_is_not_written_through() {
        let place = Place::new();
        let files = tree(&[("sub/a.md", "x")]);
        let root = place.root();
        let clear = plan(&root, &files, None)
            .into_clear()
            .expect("no collision");
        place.link("sub", &place.outside());
        let result = root.apply(&clear);
        assert!(
            matches!(result, Err(RepoError::Changed { .. })),
            "{result:?}"
        );
        assert!(snapshot(&place.outside()).is_empty());
        assert!(place.read("layer.lock").is_none());
    }

    #[test]
    fn a_fifo_or_a_folder_at_a_path_is_refused_without_blocking() {
        let place = Place::new();
        let scratch_fifo = ScratchDir::new().expect("a scratch directory");
        scratch_fifo.make_fifo("pipe").expect("a fifo");
        fs::rename(scratch_fifo.join("pipe"), place.repo().join("AGENTS.md")).expect("moved");
        fs::create_dir(place.repo().join("CLAUDE.md")).expect("made");
        place.write("x/y.md", "plain");
        // A FIFO in the middle of a path, and one in a managed folder.
        fs::create_dir_all(place.repo().join(".claude/agents")).expect("made");
        scratch_fifo.make_fifo("pipe2").expect("a fifo");
        fs::rename(
            scratch_fifo.join("pipe2"),
            place.repo().join(".claude/agents/pipe.md"),
        )
        .expect("moved");
        let files = tree(&[("AGENTS.md", "g"), ("CLAUDE.md", "g"), ("x/y.md/z.md", "g")]);
        let made = plan(&place.root(), &files, None);
        assert_eq!(
            reasons(&made),
            [
                (
                    ".claude/agents/pipe.md".to_owned(),
                    CollisionReason::NotRegular
                ),
                ("AGENTS.md".to_owned(), CollisionReason::NotRegular),
                ("CLAUDE.md".to_owned(), CollisionReason::NotRegular),
                ("x/y.md/z.md".to_owned(), CollisionReason::NotRegular),
            ]
        );
    }

    #[test]
    fn an_unmarked_file_a_lock_lists_with_another_hash_is_refused_and_left_alone() {
        let place = Place::new();
        place.write(".claude/settings.json", "{\"mine\":true}");
        let previous = previous_lock(&[(".claude/settings.json", "{}")]);
        let files = tree(&[(".claude/settings.json", "{\"alz\":true}")]);
        let before = snapshot(&place.base);
        let made = plan(&place.root(), &files, Some(&previous));
        assert_eq!(
            reasons(&made),
            [(
                ".claude/settings.json".to_owned(),
                CollisionReason::NotGenerated
            )]
        );
        assert_eq!(snapshot(&place.base), before);
    }

    #[test]
    fn an_unmarked_file_whose_hash_the_lock_recorded_is_alzs_own_and_is_replaced() {
        let place = Place::new();
        place.write(".claude/settings.json", "{}");
        let previous = previous_lock(&[(".claude/settings.json", "{}")]);
        let files = tree(&[(".claude/settings.json", "{\"alz\":true}")]);
        let root = place.root();
        let clear = plan(&root, &files, Some(&previous))
            .into_clear()
            .expect("no collision");
        let applied = root.apply(&clear).expect("applied");
        assert_eq!(
            (applied.created, applied.updated, applied.deleted),
            (0, 1, 0)
        );
        assert_eq!(
            place.read(".claude/settings.json").as_deref(),
            Some("{\"alz\":true}")
        );
    }

    #[test]
    fn only_alzs_own_files_are_deleted_and_the_folders_they_leave_empty_go() {
        let place = Place::new();
        place.write("gone/deep/owned.md", "mine");
        place.write("edited.md", "edited by hand");
        place.write("marked/a.md", &format!("<!-- {GENERATED_MARK} -->\nold"));
        place.write("shared/owned.md", "mine");
        place.write("shared/other.md", "someone else's");
        place.write("docs/guide.md", "mine");
        let previous = previous_lock(&[
            ("gone/deep/owned.md", "mine"),
            ("edited.md", "mine"),
            ("marked/a.md", "whatever"),
            ("shared/owned.md", "mine"),
            ("docs/guide.md", "mine"),
        ]);
        let root = place.root();
        let made = plan(&root, &FileTree::new(), Some(&previous));
        assert_eq!(made.changes().len(), 3, "{:?}", made.changes());
        let clear = made.into_clear().expect("no collision");
        let applied = root.apply(&clear).expect("applied");
        assert_eq!(applied.deleted, 3);
        assert!(!place.repo().join("gone").exists());
        assert!(!place.repo().join("marked").exists());
        assert_eq!(place.read("edited.md").as_deref(), Some("edited by hand"));
        assert_eq!(
            place.read("shared/other.md").as_deref(),
            Some("someone else's")
        );
        assert!(place.read("shared/owned.md").is_none());
        // A protected path is never deleted: `docs/**` is a default pattern.
        assert_eq!(place.read("docs/guide.md").as_deref(), Some("mine"));
        assert!(place.repo().exists(), "the root stays");
    }

    #[test]
    fn files_get_their_mode_whatever_the_umask_and_the_lock_is_written_last() {
        let place = Place::new();
        let mut files = tree(&[("AGENTS.md", "x")]);
        files
            .insert(
                OutputPath::try_from(".claude/hooks/run.sh").expect("a path"),
                GeneratedFile {
                    bytes: b"#!/bin/sh\n".to_vec(),
                    mode: FileMode::Executable,
                },
            )
            .expect("inserted");
        let root = place.root();
        let made = plan(&root, &files, None);
        let lock_text = made.lock_text().to_owned();
        let applied = root
            .apply(&made.into_clear().expect("clear"))
            .expect("applied");
        assert_eq!((applied.created, applied.lock_written), (2, true));
        let permissions_of = |path: &str| {
            fs::metadata(place.repo().join(path))
                .expect("exists")
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!(permissions_of("AGENTS.md"), 0o644);
        assert_eq!(permissions_of(".claude/hooks/run.sh"), 0o755);
        assert_eq!(permissions_of("layer.lock"), 0o644);
        assert_eq!(place.read("layer.lock"), Some(lock_text));
        // After an apply the plan is settled, and applying it again writes nothing.
        let again = plan(&root, &files, Some(&lock_for(&files)));
        assert!(again.is_settled(), "{again:?}");
        let applied_again = root
            .apply(&again.into_clear().expect("clear"))
            .expect("applied");
        assert_eq!(applied_again, alz_io::Applied::default());
        let leftovers = snapshot(&place.repo())
            .into_keys()
            .filter(|path| path.contains(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn a_write_that_fails_part_way_leaves_no_partial_file_and_no_lock() {
        let place = Place::new();
        place.write("b.md", &format!("<!-- {GENERATED_MARK} -->\nold b"));
        let files = tree(&[("a.md", "new a"), ("b.md", "new b")]);
        let root = place.root();
        let clear = plan(&root, &files, None)
            .into_clear()
            .expect("no collision");
        // Something replaces `b.md` with a folder after the plan: the rename over it fails.
        fs::remove_file(place.repo().join("b.md")).expect("removed");
        fs::create_dir(place.repo().join("b.md")).expect("made");
        let result = root.apply(&clear);
        assert!(matches!(result, Err(RepoError::Io { .. })), "{result:?}");
        assert_eq!(place.read("a.md").as_deref(), Some("new a"));
        assert!(place.repo().join("b.md").is_dir());
        assert!(place.read("layer.lock").is_none());
        let names: Vec<String> = snapshot(&place.repo()).into_keys().collect();
        assert_eq!(names, ["a.md", "b.md"], "no temporary file is left");
    }

    #[test]
    fn a_listed_name_that_differs_only_in_case_is_a_collision() {
        let place = Place::new();
        place.write(".claude/Settings.json", "{}");
        let files = tree(&[(".claude/settings.json", "{\"alz\":true}")]);
        let made = plan(&place.root(), &files, None);
        assert_eq!(made.collisions().len(), 1);
        assert!(matches!(
            made.collisions()
                .first()
                .map(alz_core::render::write_plan::Collision::reason),
            Some(CollisionReason::DiffersOnlyInCase(_))
        ));
    }

    #[test]
    fn a_managed_folder_is_walked_without_entering_a_link_and_a_foreign_file_is_a_collision() {
        let place = Place::new();
        fs::write(place.outside().join("secret.md"), "outside").expect("written");
        place.write(".claude/agents/mine.md", "hand written");
        place.link(".claude/agents/linked", &place.outside());
        place.write(
            ".claude/agents/nested/deep.md",
            &format!("{GENERATED_MARK}\n"),
        );
        let made = plan(&place.root(), &FileTree::new(), None);
        assert_eq!(
            reasons(&made),
            [
                (".claude/agents/linked".to_owned(), CollisionReason::Link),
                (
                    ".claude/agents/mine.md".to_owned(),
                    CollisionReason::NotGenerated
                ),
            ]
        );
    }

    #[test]
    fn the_gitignore_block_is_written_through_the_same_rules() {
        let place = Place::new();
        place.write(".gitignore", "target/");
        let files = tree(&[("AGENTS.md", "x")]);
        let root = place.root();
        let disk = root.observe(&files, None, &[]).expect("observed");
        let made = plan_writes(
            &files,
            &lock_for(&files),
            None,
            &disk,
            &Protected::new(&[], &[]),
            Generated::Ignored,
        );
        let applied = root
            .apply(&made.into_clear().expect("clear"))
            .expect("applied");
        assert!(applied.gitignore_written);
        assert_eq!(
            place.read(".gitignore").as_deref(),
            Some(
                "target/\n# >>> agent-layerz generated (do not edit this block)\n/AGENTS.md\n# <<< agent-layerz generated\n"
            )
        );
        // A link as `.gitignore` is a collision, and its target stays.
        fs::remove_file(place.repo().join(".gitignore")).expect("removed");
        fs::write(place.outside().join("ignore"), "outside").expect("written");
        place.link(".gitignore", &place.outside().join("ignore"));
        let disk = root
            .observe(&files, Some(&lock_for(&files)), &[])
            .expect("observed");
        let made = plan_writes(
            &files,
            &lock_for(&files),
            Some(&lock_for(&files)),
            &disk,
            &Protected::new(&[], &[]),
            Generated::Ignored,
        );
        assert_eq!(
            reasons(&made),
            [(".gitignore".to_owned(), CollisionReason::Link)]
        );
    }

    // The property: whatever the repository holds, applying a clear plan changes only the paths
    // the plan names (and the lock), never what is behind a link, and leaves a settled plan.

    const UNIVERSE: [&str; 4] = ["a.md", ".claude/agents/x.md", "sub/b.md", "sub/deep/c.md"];

    fn lay_out(place: &Place, kinds: &[u8]) -> Lock {
        let mut listed = Vec::new();
        for (index, (path, kind)) in UNIVERSE.iter().zip(kinds).enumerate() {
            let text = format!("plain {index}");
            match kind {
                0 => {}
                1 => {
                    place.write(path, &text);
                    listed.push((*path, text));
                }
                2 => place.write(path, &format!("{GENERATED_MARK} {index}")),
                3 => place.write(path, &text),
                4 => {
                    fs::write(place.outside().join(format!("t{index}")), "outside")
                        .expect("written");
                    place.link(path, &place.outside().join(format!("t{index}")));
                }
                _ => {
                    place.write(path, &text);
                    listed.push((*path, "another".to_owned()));
                }
            }
        }
        let files: Vec<(&str, &str)> = listed
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        previous_lock(&files)
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 48, max_global_rejects: 4096, ..ProptestConfig::default() })]

        #[test]
        fn applying_a_plan_changes_only_the_paths_it_names(
            made in prop::collection::vec(prop::option::of("[a-c]{0,3}"), UNIVERSE.len()),
            kinds in prop::collection::vec(prop::sample::select(vec![0u8, 0, 0, 1, 1, 2, 2, 3, 4, 5]), UNIVERSE.len()),
        ) {
            let place = Place::new();
            let previous = lay_out(&place, &kinds);
            let files = FileTree::from_files(made.iter().zip(UNIVERSE).filter_map(|(text, path)| {
                text.as_ref().map(|text| (OutputPath::try_from(path).expect("a path"), GeneratedFile::text(text)))
            })).expect("no collision");
            let root = place.root();
            let first = plan(&root, &files, Some(&previous));
            prop_assume!(first.collisions().is_empty());
            let named: Vec<String> = first
                .changes()
                .iter()
                .map(|change| change.path().as_str().to_owned())
                .collect();
            let before = snapshot(&place.base);
            let applied = root.apply(&first.into_clear().expect("clear"));
            prop_assert!(applied.is_ok(), "{applied:?}");
            let after = snapshot(&place.base);
            let touched = before
                .keys()
                .chain(after.keys())
                .filter(|path| before.get(*path) != after.get(*path))
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            for path in &touched {
                let relative = path.strip_prefix("repo/").unwrap_or(path);
                let is_folder_of_named = named.iter().any(|name| name.starts_with(&format!("{relative}/")));
                prop_assert!(
                    path.starts_with("repo/")
                        && (relative == "layer.lock"
                            || named.iter().any(|name| name == relative)
                            || is_folder_of_named),
                    "{path} was touched and is not named by the plan: {named:?}"
                );
            }
            let settled = plan(&root, &files, Some(&lock_for(&files)));
            prop_assert!(settled.is_settled(), "{settled:?}");
            prop_assert!(
                matches!(
                    root.apply(&settled.into_clear().expect("clear")),
                    Ok(applied) if applied == alz_io::Applied::default()
                )
            );
        }
    }

    #[test]
    fn a_change_names_the_paths_the_tests_expect() {
        // The shape of `Change` the property test relies on.
        let place = Place::new();
        let made = plan(&place.root(), &tree(&[("a.md", "x")]), None);
        assert!(matches!(made.changes(), [Change::Create { .. }]));
    }
}
