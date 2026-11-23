use proptest::prelude::{Strategy, any, prop_assert_eq, proptest};

use super::{
    AgentItems, AgentName, AgentRefusal, Isolation, Memory, OverlayMode, PermissionMode, SkillName,
};
use crate::markdown::MarkdownDoc;
use crate::merge::testing::layer;
use crate::merge::{ListEdit, OrderedSet};

fn read(files: &[(&str, &str)]) -> (AgentItems, Vec<String>) {
    let (items, problems) = AgentItems::read(&layer("personal", "personal", files));
    (items, problems.iter().map(ToString::to_string).collect())
}

fn skill(text: &str) -> SkillName {
    SkillName::try_from(text).expect("a skill name")
}

fn set(items: &[&str]) -> OrderedSet<SkillName> {
    items.iter().copied().map(skill).collect()
}

/// The edit of the `skills` field of an overlay whose frontmatter holds `field`.
fn edit_of(field: &str) -> Result<Option<ListEdit<SkillName>>, Vec<String>> {
    let doc = MarkdownDoc::parse(format!("---\n{field}\n---\n").as_bytes())
        .map_err(|error| vec![format!("{error:?}")])?;
    crate::item::list_edit::refine_edit::<SkillName>(
        &doc.front().section(),
        "skills",
        crate::merge::ListEdit::Replace,
    )
    .map_err(|errors| {
        errors
            .iter()
            .map(|error| error.message().to_string())
            .collect()
    })
}

fn apply(field: &str, base: &[&str]) -> Vec<String> {
    edit_of(field)
        .expect("an edit")
        .expect("a field")
        .apply(&set(base))
        .iter()
        .map(|skill| skill.as_str().to_owned())
        .collect()
}

#[test]
fn a_name_is_kebab_case() {
    assert!(AgentName::try_from("architect").is_ok());
    assert!(AgentName::try_from("a1-b").is_ok());
    for text in ["", "Architect", "a_b", "a--b", "-a", "a b"] {
        assert_eq!(
            AgentName::try_from(text),
            Err(AgentRefusal::Name),
            "{text:?}"
        );
    }
}

#[test]
fn the_words_of_the_closed_fields_are_those_of_the_spec_and_nothing_else() {
    assert_eq!(Isolation::try_from("worktree"), Ok(Isolation::Worktree));
    assert_eq!(Isolation::try_from("none"), Ok(Isolation::None));
    assert!(Isolation::try_from("Worktree").is_err());
    for word in ["none", "project", "user", "local"] {
        assert!(Memory::try_from(word).is_ok(), "{word}");
    }
    assert!(Memory::try_from("team").is_err());
    assert_eq!(
        PermissionMode::try_from("accept_edits"),
        Ok(PermissionMode::AcceptEdits)
    );
    assert!(PermissionMode::try_from("acceptEdits").is_err());
    assert_eq!(OverlayMode::try_from("replace"), Ok(OverlayMode::Replace));
    assert!(OverlayMode::try_from("append").is_err());
}

#[test]
fn a_base_and_an_overlay_are_read_with_their_fields() {
    let (items, problems) = read(&[
        (
            "agents/architect.md",
            "---\nname: architect\ndescription: Plans\ntools: [read, read, grep]\nskills: [a, b]\ntargets: [claude]\nclaude:\n  effort: high\n---\n\n  Plan.  \n",
        ),
        (
            "agents/architect.overlay.md",
            "---\nextends: architect\nallow_widen: true\nskills: +[c]\n---\nMore.\n",
        ),
        ("agents/notes.txt", "ignored"),
    ]);
    assert_eq!(problems, Vec::<String>::new());
    let [base] = items.bases() else {
        panic!("one base");
    };
    assert_eq!(base.name().as_str(), "architect");
    assert_eq!(base.description().as_str(), "Plans");
    assert_eq!(base.tools().map(OrderedSet::len), Some(2));
    assert_eq!(base.skills().len(), 2);
    assert_eq!(base.body(), "Plan.");
    assert!(!base.native().is_empty());
    let [overlay] = items.overlays() else {
        panic!("one overlay");
    };
    assert_eq!(overlay.extends().as_str(), "architect");
    assert_eq!(overlay.mode(), OverlayMode::Extend);
    assert!(overlay.allow_widen());
    assert_eq!(overlay.body(), "More.");
    assert!(overlay.tools().is_none());
}

#[test]
fn a_base_must_have_a_name_and_a_description_and_may_have_no_unknown_field() {
    let (items, problems) = read(&[("agents/a.md", "---\nrole: x\nname: A\nextends: a\n---\n")]);
    assert_eq!(items.bases(), []);
    assert_eq!(
        problems,
        [
            "error [agent] personal:agents/a.md:1:1: description is required",
            "error [agent] personal:agents/a.md:3:7: name must be kebab-case: lower-case letters and digits in groups joined by '-'",
            "error [agent] personal:agents/a.md:4:1: unknown field 'extends'",
        ]
    );
}

#[test]
fn an_unreadable_frontmatter_is_a_frontmatter_error() {
    let (items, problems) = read(&[("agents/a.md", "---\nname: a\n")]);
    assert_eq!(items.bases(), []);
    assert_eq!(
        problems,
        ["error [frontmatter] personal:agents/a.md: frontmatter is not closed with '---'"]
    );
}

#[test]
fn a_segment_appends_removes_or_replaces_and_segments_apply_in_the_order_written() {
    assert_eq!(apply("skills: +[c]", &["a", "b"]), ["a", "b", "c"]);
    assert_eq!(apply("skills: -[a]", &["a", "b"]), ["b"]);
    assert_eq!(apply("skills: \"[x, y]\"", &["a", "b"]), ["x", "y"]);
    assert_eq!(apply("skills: -[a] +[a]", &["a", "b"]), ["b", "a"]);
    assert_eq!(apply("skills: +[a] -[a]", &["b"]), ["b"]);
    assert_eq!(apply("skills: \"[x] +[y] -[x]\"", &["a"]), ["y"]);
    assert_eq!(
        apply("skills: \"+[\\\"q\\\", 'r' , ,s]\"", &[]),
        ["q", "r", "s"]
    );
}

#[test]
fn a_yaml_list_replaces_and_nothing_or_an_empty_string_changes_nothing() {
    assert_eq!(apply("skills: [x, y]", &["a"]), ["x", "y"]);
    assert_eq!(apply("skills: \"\"", &["a"]), ["a"]);
    assert_eq!(apply("skills: \"  \"", &["a"]), ["a"]);
    assert_eq!(edit_of("role: x"), Ok(None));
    assert_eq!(edit_of("skills: ~"), Ok(None));
}

#[test]
fn text_that_is_not_an_edit_is_refused_and_so_is_an_item_that_is_not_a_name() {
    for text in ["+a", "+[a", "[a] b", "*[a]", "+ [a]", "+[a]]"] {
        assert_eq!(
            edit_of(&format!("skills: \"{text}\"")),
            Err(vec![format!("skills {}", AgentRefusal::ListEdit.rule())]),
            "{text}"
        );
    }
    assert_eq!(
        edit_of("skills: +[good, Bad, Worse]").map_err(|errors| errors.len()),
        Err(2)
    );
    assert!(edit_of("skills: 3").is_err());
}

fn texts() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        (
            proptest::sample::select(vec!["+", "-", ""]),
            proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..4),
        ),
        0..4,
    )
    .prop_map(|segments| {
        segments
            .into_iter()
            .map(|(sign, items)| format!("{sign}[{}]", items.join(", ")))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn edit(text: &str) -> ListEdit<SkillName> {
    edit_of(&format!("skills: \"{text}\""))
        .expect("an edit")
        .expect("a field")
}

proptest! {
    #[test]
    fn writing_edits_one_after_the_other_applies_them_one_after_the_other(
        base in proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..5),
        first in texts(),
        second in texts(),
    ) {
        let base: OrderedSet<SkillName> = base.into_iter().map(skill).collect();
        let together = edit(&format!("{first} {second}")).apply(&base);
        let one_by_one = edit(&second).apply(&edit(&first).apply(&base));
        prop_assert_eq!(together, one_by_one);
    }

    #[test]
    fn composing_the_edits_of_a_stack_is_associative_and_applying_the_composition_is_applying_each(
        base in proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..5),
        a in texts(),
        b in texts(),
        c in texts(),
    ) {
        let base: OrderedSet<SkillName> = base.into_iter().map(skill).collect();
        let (a, b, c) = (edit(&a), edit(&b), edit(&c));
        let left = a.clone().merge(b.clone()).merge(c.clone());
        let right = a.clone().merge(b.clone().merge(c.clone()));
        prop_assert_eq!(left.apply(&base), right.apply(&base));
        prop_assert_eq!(&left, &right);
        prop_assert_eq!(
            left.apply(&base),
            c.apply(&b.apply(&a.apply(&base)))
        );
    }

    #[test]
    fn no_text_makes_the_segment_reader_panic(text in any::<String>()) {
        let _ = crate::item::list_edit::segments(&text);
    }
}
