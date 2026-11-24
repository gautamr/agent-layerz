use std::collections::BTreeMap;

use proptest::prelude::*;

use super::{Change, CollisionReason, Disk, MAX_COMPARED_BYTES, Observed, WritePlan, plan_writes};
use crate::layer::Visibility;
use crate::lock::{FileHash, Lock};
use crate::path::SafeRelPath;
use crate::render::gitignore::Generated;
use crate::render::header::GENERATED_MARK;
use crate::render::protected::Protected;
use crate::render::tree::{FileMode, FileTree, GeneratedFile, OutputPath};

fn safe(path: &str) -> SafeRelPath {
    SafeRelPath::try_from(path).expect("a safe path")
}

fn output(path: &str) -> OutputPath {
    OutputPath::try_from(path).expect("an output path")
}

fn tree(files: &[(&str, &str)]) -> FileTree {
    FileTree::from_files(
        files
            .iter()
            .map(|(path, text)| (output(path), GeneratedFile::text(text))),
    )
    .expect("no collision")
}

fn lock_of(files: &[(&str, &[u8])]) -> Lock {
    Lock {
        visibility: Visibility::Public,
        layers: Vec::new(),
        files: files
            .iter()
            .map(|(path, bytes)| (safe(path), FileHash::of(bytes)))
            .collect(),
    }
}

fn regular(text: &str) -> Observed {
    Observed::Regular {
        bytes: text.as_bytes().to_vec(),
        executable: false,
    }
}

fn disk(seen: Vec<(&str, Observed)>) -> Disk {
    let managed: Vec<SafeRelPath> = seen
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| path.starts_with(".claude/agents/"))
        .map(safe)
        .collect();
    Disk::new(
        seen.into_iter()
            .map(|(path, observed)| (safe(path), observed)),
        managed,
        [],
    )
}

fn plan(files: &FileTree, previous: Option<&Lock>, on_disk: &Disk) -> WritePlan {
    plan_writes(
        files,
        &lock_of(&[]),
        previous,
        on_disk,
        &Protected::new(&[], &[]),
        Generated::Committed,
    )
}

fn collision_reasons(plan: &WritePlan) -> Vec<(String, CollisionReason)> {
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

#[test]
fn a_path_that_does_not_exist_is_created() {
    let made = plan(&tree(&[("a.md", "x")]), None, &disk(vec![]));
    assert!(matches!(made.changes(), [Change::Create { .. }]));
    assert_eq!(made.collisions(), []);
}

#[test]
fn a_file_with_the_generated_mark_is_updated_when_its_bytes_differ() {
    let marked = format!("<!-- {GENERATED_MARK} -->\nold");
    let made = plan(
        &tree(&[("a.md", "new")]),
        None,
        &disk(vec![("a.md", regular(&marked))]),
    );
    assert!(matches!(made.changes(), [Change::Update { .. }]));
}

#[test]
fn a_file_with_the_same_bytes_and_mode_is_left_alone() {
    let marked = format!("<!-- {GENERATED_MARK} -->\n");
    let made = plan(
        &tree(&[("a.md", &marked)]),
        None,
        &disk(vec![("a.md", regular(&marked))]),
    );
    assert_eq!(made.changes(), []);
    assert_eq!(made.collisions(), []);
}

#[test]
fn a_difference_in_the_executable_bit_alone_is_an_update() {
    let mut files = FileTree::new();
    files
        .insert(
            output("hook.sh"),
            GeneratedFile {
                bytes: b"echo".to_vec(),
                mode: FileMode::Executable,
            },
        )
        .expect("inserted");
    let previous = lock_of(&[("hook.sh", b"echo")]);
    let made = plan(
        &files,
        Some(&previous),
        &disk(vec![("hook.sh", regular("echo"))]),
    );
    assert!(matches!(made.changes(), [Change::Update { .. }]));
}

#[test]
fn an_unmarked_file_that_nothing_lists_is_a_collision() {
    let made = plan(
        &tree(&[("a.md", "new")]),
        None,
        &disk(vec![("a.md", regular("hand written"))]),
    );
    assert_eq!(made.changes(), []);
    assert_eq!(
        collision_reasons(&made),
        [("a.md".to_owned(), CollisionReason::NotGenerated)]
    );
}

#[test]
fn a_lock_entry_with_another_hash_does_not_make_an_unmarked_file_alzs_own() {
    let previous = lock_of(&[("a.md", b"what alz wrote")]);
    let made = plan(
        &tree(&[("a.md", "new")]),
        Some(&previous),
        &disk(vec![("a.md", regular("hand edited"))]),
    );
    assert_eq!(made.changes(), []);
    assert_eq!(made.collisions().len(), 1);
}

#[test]
fn a_lock_entry_whose_hash_matches_makes_an_unmarked_file_alzs_own() {
    let previous = lock_of(&[("settings.json", b"{}")]);
    let made = plan(
        &tree(&[("settings.json", "{\"a\":1}")]),
        Some(&previous),
        &disk(vec![("settings.json", regular("{}"))]),
    );
    assert!(matches!(made.changes(), [Change::Update { .. }]));
}

#[test]
fn a_file_the_lock_lists_is_deleted_only_when_it_is_alzs_own() {
    let previous = lock_of(&[
        ("old-owned.md", b"mine"),
        ("old-edited.md", b"mine"),
        ("old-marked.md", b"anything"),
        ("old-link.md", b"mine"),
        ("old-gone.md", b"mine"),
    ]);
    let marked = format!("{GENERATED_MARK}\nedited");
    let made = plan(
        &FileTree::new(),
        Some(&previous),
        &disk(vec![
            ("old-owned.md", regular("mine")),
            ("old-edited.md", regular("edited by hand")),
            ("old-marked.md", regular(&marked)),
            ("old-link.md", Observed::Link),
        ]),
    );
    let deleted: Vec<&str> = made
        .changes()
        .iter()
        .map(|change| change.path().as_str())
        .collect();
    assert_eq!(deleted, ["old-marked.md", "old-owned.md"]);
    assert_eq!(made.collisions(), []);
}

#[test]
fn a_lock_entry_never_deletes_a_protected_path_or_the_lock() {
    let previous = lock_of(&[
        ("docs/a.md", b"x"),
        ("layer.lock", b"x"),
        (".gitignore", b"x"),
    ]);
    let made = plan_writes(
        &FileTree::new(),
        &lock_of(&[]),
        Some(&previous),
        &disk(vec![
            ("docs/a.md", regular("x")),
            ("layer.lock", regular("x")),
            (".gitignore", regular("x")),
        ]),
        &Protected::new(&[], &[]),
        Generated::Committed,
    );
    assert_eq!(made.changes(), []);
}

#[test]
fn a_path_that_is_a_link_or_lies_under_one_is_a_collision_even_when_the_lock_lists_it() {
    let previous = lock_of(&[("a.md", b"x")]);
    let made = plan(
        &tree(&[("a.md", "new"), ("b/c.md", "new"), ("d.md", "new")]),
        Some(&previous),
        &disk(vec![
            ("a.md", Observed::Link),
            ("b/c.md", Observed::Link),
            ("d.md", Observed::NotRegular),
        ]),
    );
    assert_eq!(made.changes(), []);
    assert_eq!(
        collision_reasons(&made),
        [
            ("a.md".to_owned(), CollisionReason::Link),
            ("b/c.md".to_owned(), CollisionReason::Link),
            ("d.md".to_owned(), CollisionReason::NotRegular),
        ]
    );
}

#[test]
fn a_file_that_is_too_large_to_compare_is_a_collision() {
    let made = plan(
        &tree(&[("a.md", "x")]),
        None,
        &disk(vec![("a.md", Observed::TooLarge)]),
    );
    assert_eq!(
        collision_reasons(&made),
        [("a.md".to_owned(), CollisionReason::TooLarge)]
    );
    assert!(
        made.collisions()[0]
            .reason()
            .to_string()
            .contains(&MAX_COMPARED_BYTES.to_string())
    );
}

#[test]
fn a_file_in_a_managed_folder_that_is_not_alzs_own_is_a_collision() {
    let marked = format!("{GENERATED_MARK}\n");
    let made = plan(
        &FileTree::new(),
        None,
        &disk(vec![
            (".claude/agents/mine.md", regular("hand written")),
            (".claude/agents/ours.md", regular(&marked)),
            (".claude/agents/link.md", Observed::Link),
        ]),
    );
    assert_eq!(
        collision_reasons(&made),
        [
            (".claude/agents/link.md".to_owned(), CollisionReason::Link),
            (
                ".claude/agents/mine.md".to_owned(),
                CollisionReason::NotGenerated
            ),
        ]
    );
}

#[test]
fn a_listed_name_that_differs_only_in_case_is_a_collision_unless_the_lock_lists_it() {
    let files = tree(&[(".claude/settings.json", "{}")]);
    let listed = |names: &[&str]| Disk::new([], [], names.iter().map(|name| safe(name)));
    let made = plan(&files, None, &listed(&[".claude/Settings.json"]));
    assert_eq!(made.collisions().len(), 1);
    let previous = lock_of(&[(".claude/Settings.json", b"old")]);
    let made = plan(&files, Some(&previous), &listed(&[".claude/Settings.json"]));
    assert_eq!(made.collisions(), []);
}

#[test]
fn the_lock_is_written_when_its_text_differs_and_not_otherwise() {
    let files = FileTree::new();
    let lock = lock_of(&[]);
    let text = lock.emit();
    let run = |observed: Observed| {
        plan_writes(
            &files,
            &lock,
            None,
            &Disk::new([(safe("layer.lock"), observed)], [], []),
            &Protected::new(&[], &[]),
            Generated::Committed,
        )
    };
    assert!(run(Observed::Absent).lock_changed());
    assert!(run(regular("old")).lock_changed());
    assert!(!run(regular(&text)).lock_changed());
    assert_eq!(run(Observed::Link).lock_text(), text);
    assert_eq!(run(Observed::Link).collisions().len(), 1);
}

#[test]
fn a_plan_with_a_collision_cannot_be_applied() {
    let made = plan(
        &tree(&[("a.md", "x")]),
        None,
        &disk(vec![("a.md", regular("hand written"))]),
    );
    assert_eq!(made.into_clear().map(|_| ()).unwrap_err().len(), 1);
}

#[test]
fn the_gitignore_block_is_planned_with_the_text_it_already_holds() {
    let files = tree(&[("a.md", "x")]);
    let run = |observed: Observed, generated: Generated| {
        plan_writes(
            &files,
            &lock_of(&[]),
            None,
            &Disk::new([(safe(".gitignore"), observed)], [], []),
            &Protected::new(&[], &[]),
            generated,
        )
    };
    assert!(run(Observed::Absent, Generated::Ignored).gitignore_changed());
    assert!(!run(Observed::Absent, Generated::Committed).gitignore_changed());
    assert!(!run(regular("a\n"), Generated::Committed).gitignore_changed());
    assert_eq!(
        collision_reasons(&run(
            Observed::Regular {
                bytes: vec![0xff],
                executable: false
            },
            Generated::Ignored
        )),
        [(".gitignore".to_owned(), CollisionReason::NotText)]
    );
    assert_eq!(
        collision_reasons(&run(Observed::Link, Generated::Ignored)),
        [(".gitignore".to_owned(), CollisionReason::Link)]
    );
}

#[test]
fn alzs_own_inputs_are_never_deleted_or_written_whatever_the_lock_and_the_hash_say() {
    let previous = lock_of(&[
        ("layer.yaml", b"name: x"),
        (".layer/x.md", b"mine"),
        (".Layer/y.md", b"mine"),
    ]);
    let on_disk = disk(vec![
        ("layer.yaml", regular("name: x")),
        (".layer/x.md", regular("mine")),
        (".Layer/y.md", regular("mine")),
    ]);
    let removed = plan(&FileTree::new(), Some(&previous), &on_disk);
    assert_eq!(removed.changes(), []);
    let written = plan(
        &tree(&[("layer.yaml", "x"), (".layer/z.md", "x")]),
        Some(&previous),
        &on_disk,
    );
    assert_eq!(written.changes(), []);
    assert_eq!(
        collision_reasons(&written),
        [
            (".layer/z.md".to_owned(), CollisionReason::Input),
            ("layer.yaml".to_owned(), CollisionReason::Input),
        ]
    );
}

// The property tests: a small universe of paths, every state of the disk.

const PATHS: [&str; 4] = [
    "a.md",
    ".claude/agents/x.md",
    ".claude/agents/y.md",
    "z/settings.json",
];

fn state(kind: u8, index: usize) -> (Observed, Option<FileHash>) {
    let plain = format!("plain {index}");
    let marked = format!("{GENERATED_MARK} {index}");
    match kind {
        0 => (Observed::Absent, None),
        1 => (regular(&plain), Some(FileHash::of(plain.as_bytes()))),
        2 => (regular(&marked), None),
        3 => (regular(&plain), None),
        4 => (Observed::Link, None),
        5 => (Observed::NotRegular, None),
        _ => (regular(&plain), Some(FileHash::of(b"another"))),
    }
}

fn scenario() -> impl Strategy<Value = (Vec<Option<String>>, Vec<u8>)> {
    (
        prop::collection::vec(prop::option::of("[a-c]{0,3}"), PATHS.len()),
        prop::collection::vec(
            prop::sample::select(vec![0u8, 0, 0, 0, 1, 1, 2, 2, 3, 4, 5, 6]),
            PATHS.len(),
        ),
    )
}

fn built(
    made: &[Option<String>],
    kinds: &[u8],
) -> (FileTree, Lock, BTreeMap<SafeRelPath, Observed>) {
    let files = made.iter().zip(PATHS).filter_map(|(text, path)| {
        text.as_ref()
            .map(|text| (output(path), GeneratedFile::text(text)))
    });
    let tree = FileTree::from_files(files).expect("no collision");
    let states: Vec<(Observed, Option<FileHash>)> = kinds
        .iter()
        .zip(0..)
        .map(|(kind, index)| state(*kind, index))
        .collect();
    let previous = Lock {
        visibility: Visibility::Public,
        layers: Vec::new(),
        files: PATHS
            .iter()
            .zip(&states)
            .filter_map(|(path, (_, hash))| hash.map(|hash| (safe(path), hash)))
            .collect(),
    };
    let on_disk = PATHS
        .iter()
        .zip(states)
        .map(|(path, (observed, _))| (safe(path), observed))
        .collect();
    (tree, previous, on_disk)
}

fn managed_disk(seen: &BTreeMap<SafeRelPath, Observed>) -> Disk {
    Disk::new(
        seen.clone(),
        seen.keys()
            .filter(|path| path.as_str().starts_with(".claude/agents/"))
            .cloned(),
        [],
    )
}

fn owned(observed: &Observed, hash: Option<&FileHash>) -> bool {
    match observed {
        Observed::Regular { bytes, .. } => {
            crate::render::header::has_generated_mark(bytes)
                || hash.is_some_and(|hash| *hash == FileHash::of(bytes))
        }
        Observed::Absent | Observed::Link | Observed::NotRegular | Observed::TooLarge => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { max_global_rejects: 16_384, ..ProptestConfig::default() })]

    #[test]
    fn a_change_only_ever_names_a_path_that_is_absent_or_alzs_own((made, kinds) in scenario()) {
        let (files, previous, seen) = built(&made, &kinds);
        let plan = plan_writes(
            &files, &lock_of(&[]), Some(&previous), &managed_disk(&seen),
            &Protected::new(&[], &[]), Generated::Committed,
        );
        for change in plan.changes() {
            let observed = seen.get(change.path()).expect("a path of the universe");
            let hash = previous.files.get(change.path());
            match change {
                Change::Create { .. } => prop_assert_eq!(observed, &Observed::Absent),
                Change::Update { .. } | Change::Delete { .. } => prop_assert!(owned(observed, hash)),
            }
        }
        for collision in plan.collisions() {
            let observed = seen.get(collision.path()).expect("a path of the universe");
            prop_assert!(!owned(observed, previous.files.get(collision.path())));
        }
    }

    #[test]
    fn planning_again_after_the_plan_is_applied_gives_no_changes((made, kinds) in scenario()) {
        let (files, previous, seen) = built(&made, &kinds);
        let lock = Lock {
            visibility: Visibility::Public,
            layers: Vec::new(),
            files: files
                .iter()
                .map(|(path, file)| (path.as_safe().clone(), FileHash::of(&file.bytes)))
                .collect(),
        };
        let protected = Protected::new(&[], &[]);
        let first = plan_writes(
            &files, &lock, Some(&previous), &managed_disk(&seen), &protected, Generated::Ignored,
        );
        prop_assume!(first.collisions().is_empty());
        let lock_text = first.lock_text().to_owned();
        let clear = first.into_clear().expect("no collision");
        let after = clear.changes().iter().fold(seen, |mut disk, change| {
            match change {
                Change::Create { path, file } | Change::Update { path, file } => {
                    disk.insert(
                        path.as_safe().clone(),
                        Observed::Regular {
                            bytes: file.bytes.clone(),
                            executable: file.mode == FileMode::Executable,
                        },
                    );
                }
                Change::Delete { path } => {
                    disk.insert(path.clone(), Observed::Absent);
                }
            }
            disk
        });
        let mut after = after;
        after.insert(
            safe("layer.lock"),
            Observed::Regular { bytes: lock_text.into_bytes(), executable: false },
        );
        after.insert(
            safe(".gitignore"),
            clear.gitignore().map_or(Observed::Absent, |bytes| Observed::Regular {
                bytes: bytes.to_vec(),
                executable: false,
            }),
        );
        let second = plan_writes(
            &files, &lock, Some(&lock), &managed_disk(&after), &protected, Generated::Ignored,
        );
        prop_assert!(second.is_settled(), "{:?}", second);
    }

    #[test]
    fn a_plan_is_the_same_whatever_the_order_the_disk_was_built_in((made, kinds) in scenario()) {
        let (files, previous, seen) = built(&made, &kinds);
        let forward = managed_disk(&seen);
        let backward = Disk::new(
            seen.clone().into_iter().rev(),
            seen.keys().rev().filter(|path| path.as_str().starts_with(".claude/agents/")).cloned(),
            [],
        );
        let run = |disk: &Disk| plan_writes(
            &files, &lock_of(&[]), Some(&previous), disk, &Protected::new(&[], &[]), Generated::Committed,
        );
        prop_assert_eq!(run(&forward), run(&backward));
    }
}
