use super::{InstructionEdit, InstructionItems, InstructionMode};
use crate::merge::testing::layer;

fn read(files: &[(&str, &str)]) -> (InstructionItems, Vec<String>) {
    let (items, problems) = InstructionItems::read(&layer("personal", "personal", files));
    (items, problems.iter().map(ToString::to_string).collect())
}

#[test]
fn the_modes_are_append_replace_and_remove_and_nothing_else() {
    assert_eq!(
        InstructionMode::try_from("append"),
        Ok(InstructionMode::Append)
    );
    assert_eq!(
        InstructionMode::try_from("replace"),
        Ok(InstructionMode::Replace)
    );
    assert_eq!(
        InstructionMode::try_from("remove"),
        Ok(InstructionMode::Remove)
    );
    for text in ["", "add", "Append", "Replace", "REMOVE"] {
        assert!(InstructionMode::try_from(text).is_err(), "{text:?}");
    }
}

#[test]
fn a_file_is_an_add_a_replace_or_a_remove_and_a_remove_holds_no_text() {
    let (items, problems) = read(&[
        ("instructions/a.md", "  text \n"),
        ("instructions/b.md", "---\nmode: replace\n---\nnew\n"),
        ("instructions/c.md", "---\nmode: remove\n---\nignored\n"),
    ]);
    assert_eq!(problems, Vec::<String>::new());
    let edits: Vec<&InstructionEdit> = items
        .files()
        .iter()
        .map(super::InstructionFile::edit)
        .collect();
    assert_eq!(
        edits,
        [
            &InstructionEdit::Add("text".to_owned()),
            &InstructionEdit::Replace("new".to_owned()),
            &InstructionEdit::Remove,
        ]
    );
}

#[test]
fn mode_append_adds_a_block_like_a_file_without_mode() {
    let (items, problems) = read(&[
        ("instructions/a.md", "---\nmode: append\n---\n text \n"),
        ("instructions/b.md", "text"),
    ]);
    assert_eq!(problems, Vec::<String>::new());
    let edits: Vec<&InstructionEdit> = items
        .files()
        .iter()
        .map(super::InstructionFile::edit)
        .collect();
    assert_eq!(edits, [&InstructionEdit::Add("text".to_owned()); 2]);
}

#[test]
fn only_the_files_directly_in_instructions_are_items() {
    let (items, problems) = read(&[
        ("instructions/a.md", "a"),
        ("instructions/sub/b.md", "b"),
        ("instructions/c.txt", "c"),
        ("instructions/.md", "d"),
        ("agents/e.md", "e"),
    ]);
    assert_eq!(problems, Vec::<String>::new());
    let files: Vec<String> = items.files().iter().map(ToString::to_string).collect();
    assert_eq!(files, ["instructions/a.md"]);
}

#[test]
fn a_file_with_a_problem_gives_no_item() {
    let (items, problems) = read(&[
        ("instructions/a.md", "---\ncolour: red\n---\nbody"),
        ("instructions/b.md", "fine"),
    ]);
    assert_eq!(problems.len(), 1);
    assert_eq!(items.files().len(), 1);
}

#[test]
fn a_markdown_file_that_is_not_utf8_is_a_frontmatter_error() {
    let layer = {
        use crate::layer::LayerContents;
        use crate::path::SafeRelPath;
        let base = layer("personal", "personal", &[]);
        crate::stack::ResolvedLayer::new(
            crate::stack::Source::try_from("path:../personal").expect("a source"),
            base.meta().clone(),
            [(
                SafeRelPath::try_from("instructions/a.md").expect("a path"),
                vec![0xff, 0xfe],
            )]
            .into_iter()
            .collect::<LayerContents>(),
            None,
        )
    };
    let (items, problems) = InstructionItems::read(&layer);
    assert_eq!(items.files().len(), 0);
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        ["error [frontmatter] personal:instructions/a.md: the file is not valid UTF-8"]
    );
}
