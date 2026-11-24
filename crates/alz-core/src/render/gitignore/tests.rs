use super::{BLOCK_END, BLOCK_START, Generated, synced};
use crate::render::tree::{FileTree, GeneratedFile, OutputPath};

fn tree(paths: &[&str]) -> FileTree {
    FileTree::from_files(paths.iter().map(|path| {
        (
            OutputPath::try_from(*path).expect("a path"),
            GeneratedFile::text("x"),
        )
    }))
    .expect("no collision")
}

#[test]
fn a_new_block_lists_each_generated_path_in_name_order() {
    let text = synced(
        "",
        &tree(&["b.md", "a.md", ".claude/x.md"]),
        Generated::Ignored,
    );
    assert_eq!(
        text,
        format!("{BLOCK_START}\n/.claude/x.md\n/a.md\n/b.md\n{BLOCK_END}\n")
    );
}

#[test]
fn a_line_feed_is_added_before_the_block_when_the_text_does_not_end_with_one() {
    let expected = format!("node_modules\n{BLOCK_START}\n/a.md\n{BLOCK_END}\n");
    assert_eq!(
        synced("node_modules", &tree(&["a.md"]), Generated::Ignored),
        expected
    );
    assert_eq!(
        synced("node_modules\n", &tree(&["a.md"]), Generated::Ignored),
        expected
    );
}

#[test]
fn text_outside_the_block_is_kept_byte_for_byte() {
    let before = format!("a\r\n\n{BLOCK_START}\n/old\n{BLOCK_END}\n  tail without end");
    let text = synced(&before, &tree(&["a.md"]), Generated::Ignored);
    assert_eq!(
        text,
        format!("a\r\n\n  tail without end\n{BLOCK_START}\n/a.md\n{BLOCK_END}\n")
    );
}

#[test]
fn a_block_under_the_earlier_name_is_replaced() {
    let legacy = "keep\n# >>> agent-layers generated (do not edit this block)\n/old\n# <<< agent-layers generated\nafter\n";
    let text = synced(legacy, &tree(&["a.md"]), Generated::Ignored);
    assert_eq!(
        text,
        format!("keep\nafter\n{BLOCK_START}\n/a.md\n{BLOCK_END}\n")
    );
}

#[test]
fn committing_the_files_removes_the_block_and_nothing_else() {
    let before = format!("keep\n{BLOCK_START}\n/a.md\n{BLOCK_END}\nafter\n");
    assert_eq!(
        synced(&before, &tree(&["a.md"]), Generated::Committed),
        "keep\nafter\n"
    );
    assert_eq!(
        synced("keep\n", &tree(&["a.md"]), Generated::Committed),
        "keep\n"
    );
}

#[test]
fn a_block_without_its_end_line_is_not_a_block() {
    let before = format!("{BLOCK_START}\n/old\n");
    let text = synced(&before, &tree(&["a.md"]), Generated::Ignored);
    assert_eq!(text, format!("{before}{BLOCK_START}\n/a.md\n{BLOCK_END}\n"));
}

#[test]
fn syncing_twice_changes_nothing_the_second_time() {
    let files = tree(&["a.md", "b.md"]);
    let once = synced("x\n", &files, Generated::Ignored);
    assert_eq!(synced(&once, &files, Generated::Ignored), once);
}

#[test]
fn an_empty_tree_leaves_the_empty_line_the_reference_writes() {
    assert_eq!(
        synced("", &FileTree::new(), Generated::Ignored),
        format!("{BLOCK_START}\n\n{BLOCK_END}\n")
    );
}
