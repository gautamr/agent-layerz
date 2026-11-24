use crate::layer::Visibility;
use crate::lock::Lock;
use crate::merge::testing::{layer, layer_visible};
use crate::render::tree::{FileMode, FileTree, GeneratedFile, OutputPath};
use crate::stack::ResolvedStack;

use super::lock_of;

const REPOSITORY_LOCK: &[u8] = include_bytes!("../../../../../layer.lock");
const AGENTS_MD: &[u8] = include_bytes!("../../../../../AGENTS.md");
const CLAUDE_MD: &[u8] = include_bytes!("../../../../../CLAUDE.md");
const MCP_JSON: &[u8] = include_bytes!("../../../../../.mcp.json");

fn tree(files: &[(&str, &[u8])]) -> FileTree {
    FileTree::from_files(files.iter().map(|(path, bytes)| {
        (
            OutputPath::try_from(*path).expect("an output path"),
            GeneratedFile {
                bytes: bytes.to_vec(),
                mode: FileMode::Regular,
            },
        )
    }))
    .expect("no collisions")
}

fn stack() -> ResolvedStack {
    ResolvedStack::validate(vec![
        layer_visible("personal", "personal", "public", &[]),
        layer("shop-api", "repo", &[("instructions/10.md", "x")]),
    ])
    .expect("a valid stack")
}

#[test]
fn the_hash_of_each_file_is_the_one_the_reference_wrote_into_this_repositorys_lock() {
    let reference = Lock::parse(REPOSITORY_LOCK).expect("the repository's lock");
    let lock = lock_of(
        &tree(&[
            ("AGENTS.md", AGENTS_MD),
            ("CLAUDE.md", CLAUDE_MD),
            (".mcp.json", MCP_JSON),
        ]),
        &stack(),
    );
    assert_eq!(lock.files.len(), 3);
    for (path, hash) in &lock.files {
        assert_eq!(reference.files.get(path), Some(hash), "{path}");
    }
}

#[test]
fn the_lock_takes_the_most_private_visibility_and_each_layer_in_stack_order() {
    let lock = lock_of(&FileTree::new(), &stack());
    assert_eq!(lock.visibility, Visibility::Private);
    assert_eq!(
        lock.layers
            .iter()
            .map(|layer| layer.name.to_string())
            .collect::<Vec<_>>(),
        ["personal", "shop-api"]
    );
    assert!(
        lock.layers
            .iter()
            .all(|layer| layer.rev.is_none() && layer.sha.is_none()),
        "a layer read in place has no pin"
    );
    assert!(lock.files.is_empty());
}

#[test]
fn the_lock_is_emitted_the_same_every_time_and_reads_back() {
    let files = tree(&[("b.md", b"b"), ("a.md", b"a")]);
    let first = lock_of(&files, &stack()).emit();
    assert_eq!(first, lock_of(&files, &stack()).emit());
    let parsed = Lock::parse(first.as_bytes()).expect("a lock reads back");
    assert_eq!(parsed.emit(), first);
    assert!(
        first.find("\"a.md\"") < first.find("\"b.md\""),
        "files are in name order"
    );
}
