use super::{Skill, SkillItems};
use crate::item::SkillName;
use crate::merge::testing::{layer, layer_from_yaml};
use crate::stack::ResolvedLayer;

const SKILL: &str = "---\nname: glossary\ndescription: Terms\n---\nbody\n";

fn read(layer: &ResolvedLayer) -> (SkillItems, Vec<String>) {
    let (items, found) = SkillItems::read(layer);
    (items, found.iter().map(ToString::to_string).collect())
}

fn one(files: &[(&str, &str)]) -> (SkillItems, Vec<String>) {
    read(&layer("personal", "personal", files))
}

fn names(items: &SkillItems) -> Vec<&str> {
    items
        .skills()
        .iter()
        .map(|skill| skill.name().as_str())
        .collect()
}

#[test]
fn a_skill_is_its_directory_with_every_file_and_its_bytes() {
    let (items, found) = one(&[
        ("skills/glossary/SKILL.md", SKILL),
        ("skills/glossary/references/terms.md", "t"),
        ("skills/glossary/scripts/run.sh", "echo"),
        ("skills/other/notes.md", "not a skill"),
        ("README.md", "no item"),
    ]);
    assert_eq!(
        found,
        ["error [skill] personal:skills/other: skill directory 'other' has no SKILL.md"]
    );
    let [Skill::Directory(directory)] = items.skills() else {
        panic!("one skill directory");
    };
    assert_eq!(directory.description().as_str(), "Terms");
    let files: Vec<(String, &[u8])> = directory
        .files()
        .iter()
        .map(|file| (file.within().collect::<Vec<_>>().join("/"), file.bytes()))
        .collect();
    assert_eq!(
        files,
        [
            ("SKILL.md".to_owned(), SKILL.as_bytes()),
            ("references/terms.md".to_owned(), b"t".as_slice()),
            ("scripts/run.sh".to_owned(), b"echo".as_slice()),
        ]
    );
}

#[test]
fn the_debug_output_of_a_skill_file_holds_its_size_and_never_its_bytes() {
    let (items, _) = one(&[("skills/glossary/SKILL.md", SKILL)]);
    assert!(!format!("{items:?}").contains("body"));
}

#[test]
fn a_skill_md_name_that_differs_from_its_directory_is_an_error() {
    let (items, found) = one(&[(
        "skills/glossary/SKILL.md",
        "---\nname: terms\ndescription: Terms\n---\n",
    )]);
    assert_eq!(
        found,
        [
            "error [skill] personal:skills/glossary/SKILL.md: name must match the directory name, glossary"
        ]
    );
    assert_eq!(items.skills(), []);
}

#[test]
fn a_skill_directory_without_skill_md_is_an_error() {
    let (items, found) = one(&[("skills/glossary/references/a.md", "x")]);
    assert_eq!(
        found,
        ["error [skill] personal:skills/glossary: skill directory 'glossary' has no SKILL.md"]
    );
    assert_eq!(items.skills(), []);
}

#[test]
fn a_directory_name_that_is_not_kebab_case_is_an_error() {
    let (items, found) = one(&[(
        "skills/Stack_Conventions/SKILL.md",
        "---\nname: stack-conventions\ndescription: x\n---\n",
    )]);
    assert_eq!(
        found,
        [
            "error [skill] personal:skills/Stack_Conventions/SKILL.md: skill name 'Stack_Conventions' must be kebab-case: lower-case letters and digits in groups joined by '-'"
        ]
    );
    assert_eq!(items.skills(), []);
}

#[test]
fn a_skill_without_a_description_or_a_name_is_an_error_at_its_skill_md() {
    for (front, expected) in [
        (
            "name: glossary",
            "error [skill] personal:skills/glossary/SKILL.md:1:1: description is required",
        ),
        (
            "name: glossary\ndescription: \"  \"",
            "error [skill] personal:skills/glossary/SKILL.md:3:14: description must not be empty",
        ),
        (
            "description: Terms",
            "error [skill] personal:skills/glossary/SKILL.md:1:1: name is required",
        ),
    ] {
        let text = format!("---\n{front}\n---\n");
        let (items, found) = one(&[("skills/glossary/SKILL.md", &text)]);
        assert_eq!(items.skills(), [], "{front}");
        assert_eq!(found, [expected], "{front}");
    }
}

#[test]
fn a_skill_md_with_unclosed_frontmatter_is_a_frontmatter_error() {
    let (items, found) = one(&[("skills/glossary/SKILL.md", "---\nname: glossary\n")]);
    assert_eq!(
        found,
        [
            "error [frontmatter] personal:skills/glossary/SKILL.md: frontmatter is not closed with '---'"
        ]
    );
    assert_eq!(items.skills(), []);
}

#[test]
fn other_fields_of_a_skill_md_pass_through() {
    let (items, found) = one(&[(
        "skills/glossary/SKILL.md",
        "---\nname: glossary\ndescription: x\nallowed-tools: Read\nmetadata: {a: 1}\n---\n",
    )]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(names(&items), ["glossary"]);
}

#[test]
fn a_command_is_a_file_with_a_description_and_an_optional_argument_hint() {
    let (items, found) = one(&[(
        "commands/release.md",
        "---\ndescription: Ship it\nargument_hint: <version>\n---\n\nRun the release.\n",
    )]);
    assert_eq!(found, Vec::<String>::new());
    let [Skill::Command(command)] = items.skills() else {
        panic!("one command");
    };
    assert_eq!(command.name().as_str(), "release");
    assert_eq!(command.description().as_str(), "Ship it");
    assert_eq!(
        command.argument_hint().map(super::ArgumentHint::as_str),
        Some("<version>")
    );
    assert_eq!(command.body(), "Run the release.");
}

#[test]
fn a_command_with_an_unknown_field_a_missing_description_or_a_bad_name_is_an_error() {
    let (items, found) = one(&[
        ("commands/a.md", "---\ndescription: x\nmodel: y\n---\n"),
        ("commands/b.md", "body only\n"),
        ("commands/Bad_Name.md", "---\ndescription: x\n---\n"),
    ]);
    assert_eq!(
        found,
        [
            "error [command] personal:commands/Bad_Name.md: command name 'Bad_Name' must be kebab-case: lower-case letters and digits in groups joined by '-'",
            "error [command] personal:commands/a.md:3:1: unknown field 'model'",
            "error [command] personal:commands/b.md:1:1: description is required",
        ]
    );
    assert_eq!(items.skills(), []);
}

#[test]
fn a_command_and_a_skill_with_one_name_in_one_layer_are_an_error() {
    let (items, found) = one(&[
        ("skills/glossary/SKILL.md", SKILL),
        ("commands/glossary.md", "---\ndescription: x\n---\n"),
        ("commands/release.md", "---\ndescription: x\n---\n"),
    ]);
    assert_eq!(
        found,
        [
            "error [skill-command-clash] personal:commands/glossary.md: glossary is both a skill and a command in this layer"
        ]
    );
    assert_eq!(names(&items), ["glossary", "release"]);
    assert!(matches!(items.skills()[0], Skill::Directory(_)));
}

#[test]
fn a_provides_slots_entry_without_its_skill_is_an_error() {
    let yaml = "name: personal\nkind: personal\nvisibility: public\nprovides_slots: [glossary, adr, release]\n";
    let one_layer = layer_from_yaml(
        yaml,
        &[
            ("skills/glossary/SKILL.md", SKILL),
            ("commands/release.md", "---\ndescription: x\n---\n"),
        ],
    );
    let (items, found) = SkillItems::read(&one_layer);
    assert_eq!(found.error_count(), 0);
    let checked: Vec<String> = items
        .check_provided(one_layer.meta())
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        checked,
        [
            "error [slot] personal:layer.yaml: provides_slots lists adr, but the layer has no such skill"
        ]
    );
}

#[test]
fn a_provides_slots_entry_whose_skill_has_an_error_is_reported_without_a_skill() {
    let yaml = "name: personal\nkind: personal\nvisibility: public\nprovides_slots: [glossary]\n";
    let one_layer = layer_from_yaml(
        yaml,
        &[("skills/glossary/SKILL.md", "---\nname: glossary\n---\n")],
    );
    let (items, found) = SkillItems::read(&one_layer);
    assert_eq!(found.error_count(), 1);
    assert_eq!(items.check_provided(one_layer.meta()).error_count(), 1);
}

#[test]
fn a_requires_slots_entry_that_is_not_a_skill_name_is_an_error() {
    let yaml = "name: personal\nkind: personal\nvisibility: public\nrequires_slots: [domain-glossary, Bad Name]\n";
    let (items, found) = read(&layer_from_yaml(yaml, &[]));
    assert_eq!(
        found,
        [
            "error [slot] personal:layer.yaml: requires_slots entry Bad Name is not a kebab-case skill name"
        ]
    );
    assert_eq!(
        items
            .requires()
            .iter()
            .map(SkillName::as_str)
            .collect::<Vec<_>>(),
        ["domain-glossary"]
    );
}

#[test]
fn an_untrusted_name_in_a_message_is_quoted_and_stripped() {
    let (_, found) = one(&[("skills/a\u{202e}b/notes.md", "x")]);
    assert_eq!(
        found,
        ["error [skill] personal:skills/ab: skill directory 'ab' has no SKILL.md"]
    );
}
