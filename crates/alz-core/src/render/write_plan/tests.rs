use std::collections::BTreeMap;

use proptest::prelude::*;

use super::{
    Change, CollisionReason, Disk, Inputs, MAX_COMPARED_BYTES, Observed, WritePlan, plan_writes,
};
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
        &Inputs::default(),
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
        &Inputs::default(),
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
fn a_listed_name_that_differs_only_in_case_is_a_collision_unless_the_lock_lists_it_with_its_hash() {
    let files = tree(&[(".claude/settings.json", "{}")]);
    let listed = |names: &[&str], content: &str| {
        Disk::new(
            names.iter().map(|name| (safe(name), regular(content))),
            [],
            names.iter().map(|name| safe(name)),
        )
    };
    let made = plan(&files, None, &listed(&[".claude/Settings.json"], "old"));
    assert_eq!(made.collisions().len(), 1);
    let previous = lock_of(&[(".claude/Settings.json", b"old")]);
    let made = plan(
        &files,
        Some(&previous),
        &listed(&[".claude/Settings.json"], "old"),
    );
    assert_eq!(made.collisions(), []);
    let made = plan(
        &files,
        Some(&previous),
        &listed(&[".claude/Settings.json"], "edited"),
    );
    assert_eq!(
        made.collisions().len(),
        1,
        "the lock's hash no longer matches"
    );
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
            &Inputs::default(),
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
            &Inputs::default(),
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

#[test]
fn the_lock_and_the_ignore_file_are_reserved_after_case_folding_as_the_inputs_are() {
    let names = [
        ".GITIGNORE",
        "Layer.Lock",
        "LAYER.YAML",
        ".LAYER/x.md",
        // NFD folding: a composed and a decomposed spelling are one name.
        ".g\u{69}tignore",
    ];
    let previous = lock_of(&names.map(|name| (name, &b"mine"[..])));
    let on_disk = disk(names.iter().map(|name| (*name, regular("mine"))).collect());
    let removed = plan(&FileTree::new(), Some(&previous), &on_disk);
    assert_eq!(removed.changes(), []);
}

#[test]
fn every_repository_layer_folder_is_an_input_not_only_the_layer_folder() {
    let inputs = Inputs::new([safe("team/rules"), safe("Docs")]);
    let previous = lock_of(&[
        ("team/rules/a.md", b"mine"),
        ("TEAM/Rules/b.md", b"mine"),
        ("docs/c.md", b"mine"),
        ("team/other.md", b"mine"),
    ]);
    let on_disk = disk(vec![
        ("team/rules/a.md", regular("mine")),
        ("TEAM/Rules/b.md", regular("mine")),
        ("docs/c.md", regular("mine")),
        ("team/other.md", regular("mine")),
    ]);
    let run = |files: &FileTree| {
        plan_writes(
            files,
            &lock_of(&[]),
            Some(&previous),
            &on_disk,
            &Protected::new(&[], &[]),
            &inputs,
            Generated::Committed,
        )
    };
    let removed = run(&FileTree::new());
    let deleted: Vec<&str> = removed
        .changes()
        .iter()
        .map(|c| c.path().as_str())
        .collect();
    assert_eq!(
        deleted,
        ["team/other.md"],
        "only a path outside every input goes"
    );
    let written = run(&tree(&[("team/rules/new.md", "x"), ("docs", "x")]));
    assert_eq!(
        collision_reasons(&written),
        [
            ("docs".to_owned(), CollisionReason::Input),
            ("team/rules/new.md".to_owned(), CollisionReason::Input),
        ]
    );
}

/// A small file system for the case-only rename: case sensitive or not, with `apply` that fails
/// where the real writer does (a create over an existing name, a delete of a missing one is fine).
#[derive(Clone)]
struct Fs {
    folds: bool,
    files: BTreeMap<String, (String, Vec<u8>)>,
}

impl Fs {
    fn key(&self, name: &str) -> String {
        if self.folds {
            name.to_ascii_lowercase()
        } else {
            name.to_owned()
        }
    }

    fn put(&mut self, name: &str, bytes: &[u8]) {
        let key = self.key(name);
        self.files.insert(key, (name.to_owned(), bytes.to_vec()));
    }

    fn disk(&self, files: &FileTree, previous: &Lock) -> Disk {
        let wanted = files
            .paths()
            .map(|path| path.as_safe().clone())
            .chain(previous.files.keys().cloned())
            .chain([safe("layer.lock"), safe(".gitignore")]);
        let seen = wanted.map(|path| {
            let observed =
                self.files
                    .get(&self.key(path.as_str()))
                    .map_or(Observed::Absent, |(_, bytes)| Observed::Regular {
                        bytes: bytes.clone(),
                        executable: false,
                    });
            (path, observed)
        });
        let planned: Vec<String> = files.paths().map(OutputPath::folded).collect();
        let listed = self
            .files
            .values()
            .map(|(name, _)| safe(name))
            .filter(|name| planned.contains(&crate::render::tree::fold(name)));
        Disk::new(seen, [], listed)
    }

    /// Applies the changes in order; `Err` is what the real writer reports as changed.
    fn apply(&mut self, plan: &WritePlan) -> Result<(), String> {
        for change in plan.changes() {
            match change {
                Change::Create { path, file } => {
                    if self.files.contains_key(&self.key(path.as_str())) {
                        return Err(format!("{} appeared", path.as_str()));
                    }
                    self.put(path.as_str(), &file.bytes);
                }
                Change::Update {
                    path,
                    file,
                    observed,
                } => {
                    if self
                        .files
                        .get(&self.key(path.as_str()))
                        .map(|(_, bytes)| bytes)
                        != Some(observed)
                    {
                        return Err(format!("{} changed", path.as_str()));
                    }
                    self.put(path.as_str(), &file.bytes);
                }
                Change::Delete { path, observed } => match self.files.get(&self.key(path.as_str()))
                {
                    Some((_, bytes)) if bytes == observed => {
                        let key = self.key(path.as_str());
                        self.files.remove(&key);
                    }
                    Some(_) => return Err(format!("{} changed", path.as_str())),
                    None => {}
                },
            }
        }
        self.put("layer.lock", plan.lock_text().as_bytes());
        Ok(())
    }
}

fn rename_run(
    folds: bool,
    new_exists: Option<&[u8]>,
    old: &[u8],
) -> (Fs, FileTree, Lock, WritePlan) {
    let previous = lock_of(&[("docs/readme.md", b"old text")]);
    let mut fs = Fs {
        folds,
        files: BTreeMap::new(),
    };
    fs.put("docs/readme.md", old);
    if let Some(bytes) = new_exists {
        fs.put("docs/README.md", bytes);
    }
    let files = tree(&[("docs/README.md", "new text")]);
    let made = plan_writes(
        &files,
        &lock_of(&[("docs/README.md", b"new text")]),
        Some(&previous),
        &fs.disk(&files, &previous),
        &Protected::new(&[], &[]),
        &Inputs::default(),
        Generated::Committed,
    );
    (fs, files, previous, made)
}

#[test]
fn a_case_only_rename_of_a_generated_file_is_one_build_on_either_kind_of_file_system() {
    for folds in [false, true] {
        let (mut fs, files, _, made) = rename_run(folds, None, b"old text");
        assert_eq!(made.collisions(), [], "folds: {folds}");
        let order: Vec<&str> = made
            .changes()
            .iter()
            .map(|change| match change {
                Change::Delete { .. } => "delete",
                Change::Create { .. } => "create",
                Change::Update { .. } => "update",
            })
            .collect();
        assert_eq!(
            order,
            ["delete", "create"],
            "the old name goes before the new is made"
        );
        fs.apply(&made).expect("applies");
        let names: Vec<&str> = fs.files.values().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["docs/README.md", "layer.lock"], "folds: {folds}");
        let now = lock_of(&[("docs/README.md", b"new text")]);
        let again = plan_writes(
            &files,
            &now,
            Some(&now),
            &fs.disk(&files, &now),
            &Protected::new(&[], &[]),
            &Inputs::default(),
            Generated::Committed,
        );
        assert!(
            again.changes().is_empty() && again.collisions().is_empty(),
            "{again:?}"
        );
    }
}

#[test]
fn a_case_variant_the_lock_lists_is_deleted_only_while_it_holds_what_the_lock_recorded() {
    for folds in [false, true] {
        let (_, _, _, made) = rename_run(folds, None, b"edited by hand");
        assert_eq!(
            collision_reasons(&made),
            [(
                "docs/README.md".to_owned(),
                CollisionReason::DiffersOnlyInCase(safe("docs/readme.md"))
            )],
            "folds: {folds}"
        );
        assert_eq!(made.changes(), []);
    }
}

#[test]
fn a_case_variant_beside_a_new_name_that_exists_separately_is_deleted_and_the_new_one_updated() {
    let (fs, _, _, made) = rename_run(false, Some(b"stale generated"), b"old text");
    // The new name holds something alz did not write: it is not alz's own.
    assert_eq!(made.collisions().len(), 1);
    let marked = format!("<!-- {GENERATED_MARK} -->\nstale");
    let (mut fs_marked, _, _, made) = rename_run(false, Some(marked.as_bytes()), b"old text");
    assert_eq!(made.collisions(), []);
    fs_marked.apply(&made).expect("applies");
    assert_eq!(fs_marked.files.len(), 2, "README.md and layer.lock");
    assert_eq!(fs.files.len(), 2, "unchanged by a refused plan");
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
            &Protected::new(&[], &[]), &Inputs::default(), Generated::Committed,
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
            &files, &lock, Some(&previous), &managed_disk(&seen), &protected, &Inputs::default(), Generated::Ignored,
        );
        prop_assume!(first.collisions().is_empty());
        let lock_text = first.lock_text().to_owned();
        let clear = first.into_clear().expect("no collision");
        let after = clear.changes().iter().fold(seen, |mut disk, change| {
            match change {
                Change::Create { path, file } | Change::Update { path, file, .. } => {
                    disk.insert(
                        path.as_safe().clone(),
                        Observed::Regular {
                            bytes: file.bytes.clone(),
                            executable: file.mode == FileMode::Executable,
                        },
                    );
                }
                Change::Delete { path, .. } => {
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
            &files, &lock, Some(&lock), &managed_disk(&after), &protected, &Inputs::default(), Generated::Ignored,
        );
        prop_assert!(second.is_settled(), "{:?}", second);
    }

    #[test]
    fn a_case_only_rename_settles_in_one_build_whatever_the_file_system_and_the_files(
        folds in any::<bool>(),
        old_kind in 0u8..3,
        new_kind in 0u8..4,
    ) {
        let marked = format!("<!-- {GENERATED_MARK} -->\nstale");
        let old: Vec<u8> = match old_kind {
            0 => b"old text".to_vec(),
            1 => b"edited by hand".to_vec(),
            _ => marked.clone().into_bytes(),
        };
        // On a file system that ignores case the new spelling cannot exist beside the old one.
        let separate: Option<Vec<u8>> = match new_kind {
            0 => None,
            1 => Some(marked.clone().into_bytes()),
            2 => Some(b"hand written".to_vec()),
            _ => Some(b"old text".to_vec()),
        };
        prop_assume!(!folds || separate.is_none());
        let (mut fs, files, _, made) = rename_run(folds, separate.as_deref(), &old);
        if made.collisions().is_empty() {
            prop_assert_eq!(fs.apply(&made), Ok(()), "{:?}", made);
            let now = lock_of(&[("docs/README.md", b"new text")]);
            let again = plan_writes(
                &files, &now, Some(&now), &fs.disk(&files, &now),
                &Protected::new(&[], &[]), &Inputs::default(), Generated::Committed,
            );
            prop_assert!(again.changes().is_empty() && again.collisions().is_empty(), "{:?}", again);
            let names: Vec<&str> = fs.files.values().map(|(name, _)| name.as_str()).collect();
            prop_assert_eq!(names, ["docs/README.md", "layer.lock"]);
        } else {
            // A refusal never lets a file that is not alz's own be deleted or replaced.
            prop_assert!(made.into_clear().is_err());
        }
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
            &files, &lock_of(&[]), Some(&previous), disk, &Protected::new(&[], &[]), &Inputs::default(), Generated::Committed,
        );
        prop_assert_eq!(run(&forward), run(&backward));
    }
}
