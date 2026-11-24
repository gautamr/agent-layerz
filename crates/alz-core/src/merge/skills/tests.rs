use std::collections::BTreeMap;

use proptest::collection::{btree_map, btree_set};
use proptest::prelude::{Strategy, prop_assert_eq, proptest};

use crate::item::{Skill, SkillName};
use crate::merge::action::Action;
use crate::merge::provenance::Traced;
use crate::merge::testing::{layer, layer_from_yaml, lines, merged, merged_with};
use crate::merge::{Merged, SlotDemand};
use crate::stack::ResolvedLayer;

// The files of acceptance case 05 (§13.5), read from the fixture itself.
const CASE_05_PERSONAL_YAML: &str =
    include_str!("../../../../../fixtures/05-skill-override/personal/layer.yaml");
const CASE_05_PERSONAL_SKILL: &str = include_str!(
    "../../../../../fixtures/05-skill-override/personal/skills/stack-conventions/SKILL.md"
);
const CASE_05_REPO_YAML: &str =
    include_str!("../../../../../fixtures/05-skill-override/repo/.layer/layer.yaml");
const CASE_05_REPO_SKILL: &str = include_str!(
    "../../../../../fixtures/05-skill-override/repo/.layer/skills/stack-conventions/SKILL.md"
);

// The layers of acceptance case 06 (§13.6): the personal layer requires `domain-glossary`.
const CASE_06_PERSONAL_YAML: &str =
    include_str!("../../../../../fixtures/06-slots/personal/layer.yaml");
const CASE_06_REPO_YAML: &str =
    include_str!("../../../../../fixtures/06-slots/repo-strict/.layer/layer.yaml");
const LOOSE: &str = "options:\n  strict_slots: false\n";
const STRICT: &str = "options:\n  strict_slots: true\n";

fn case_05() -> Vec<ResolvedLayer> {
    vec![
        layer_from_yaml(
            CASE_05_PERSONAL_YAML,
            &[("skills/stack-conventions/SKILL.md", CASE_05_PERSONAL_SKILL)],
        ),
        layer_from_yaml(
            CASE_05_REPO_YAML,
            &[("skills/stack-conventions/SKILL.md", CASE_05_REPO_SKILL)],
        ),
    ]
}

fn case_06() -> Vec<ResolvedLayer> {
    vec![
        layer_from_yaml(CASE_06_PERSONAL_YAML, &[]),
        layer_from_yaml(CASE_06_REPO_YAML, &[]),
    ]
}

fn skill_name(name: &str) -> SkillName {
    SkillName::try_from(name).expect("a skill name")
}

fn skill<'a>(model: &'a Merged, name: &str) -> &'a Traced<Skill> {
    model
        .skills()
        .get(&skill_name(name))
        .expect("the skill exists")
}

/// The paths of a skill's files inside its directory, `SKILL.md` and the rest.
fn files_of(skill: &Skill) -> Vec<String> {
    match skill {
        Skill::Directory(directory) => directory
            .files()
            .iter()
            .map(|file| file.within().collect::<Vec<_>>().join("/"))
            .collect(),
        Skill::Command(_) => Vec::new(),
    }
}

fn layers_of(traced: &Traced<Skill>) -> Vec<(String, Action)> {
    traced
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str().to_owned(), part.action().clone()))
        .collect()
}

fn model(layers: Vec<ResolvedLayer>) -> Merged {
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    found.merged().expect("no error was found").clone()
}

const SKILL: &str = "---\nname: glossary\ndescription: Terms\n---\nbody\n";

#[test]
fn case_05_the_repo_skill_replaces_the_personal_one() {
    let model = model(case_05());
    let traced = skill(&model, "stack-conventions");
    let Skill::Directory(directory) = traced.value() else {
        panic!("a skill directory");
    };
    let text = std::str::from_utf8(directory.files()[0].bytes()).expect("text");
    assert!(text.contains("REPO-STACK"));
    assert!(!text.contains("PERSONAL-STACK"));
    assert_eq!(
        directory.description().as_str(),
        "Conventions of the shop-api stack"
    );
    assert_eq!(model.skills().len(), 1);
}

#[test]
fn case_05_the_provenance_lists_the_overridden_layer_first() {
    let model = model(case_05());
    assert_eq!(
        layers_of(skill(&model, "stack-conventions")),
        [
            ("personal".to_owned(), Action::Define),
            ("shop-api".to_owned(), Action::Replace)
        ]
    );
}

#[test]
fn the_whole_directory_is_replaced_and_files_of_the_older_one_are_dropped() {
    let general = "---\nname: arch\ndescription: General\n---\n";
    let specific = "---\nname: arch\ndescription: Specific\n---\n";
    let model = model(vec![
        layer(
            "personal",
            "personal",
            &[
                ("skills/arch/SKILL.md", general),
                ("skills/arch/references/old.md", "old"),
                ("skills/arch/scripts/run.sh", "echo"),
            ],
        ),
        layer(
            "repo",
            "repo",
            &[
                ("skills/arch/SKILL.md", specific),
                ("skills/arch/references/new.md", "new"),
            ],
        ),
    ]);
    assert_eq!(
        files_of(skill(&model, "arch").value()),
        ["SKILL.md", "references/new.md"]
    );
}

#[test]
fn a_more_general_layers_skill_survives_when_no_other_layer_defines_that_name() {
    let model = model(vec![
        layer(
            "personal",
            "personal",
            &[("skills/glossary/SKILL.md", SKILL)],
        ),
        layer("repo", "repo", &[]),
    ]);
    assert_eq!(
        layers_of(skill(&model, "glossary")),
        [("personal".to_owned(), Action::Define)]
    );
}

#[test]
fn a_command_replaces_a_skill_of_a_more_general_layer_and_the_reverse() {
    let command = "---\ndescription: Do it\n---\nrun\n";
    let model = model(vec![
        layer(
            "personal",
            "personal",
            &[
                ("skills/glossary/SKILL.md", SKILL),
                ("commands/release.md", command),
            ],
        ),
        layer(
            "repo",
            "repo",
            &[
                ("commands/glossary.md", command),
                (
                    "skills/release/SKILL.md",
                    "---\nname: release\ndescription: Release\n---\n",
                ),
            ],
        ),
    ]);
    assert!(matches!(
        skill(&model, "glossary").value(),
        Skill::Command(_)
    ));
    assert!(matches!(
        skill(&model, "release").value(),
        Skill::Directory(_)
    ));
}

#[test]
fn case_06_a_missing_slot_fails_with_strict_slots() {
    let found = merged_with(case_06(), STRICT);
    assert_eq!(
        lines(&found),
        [
            "error [slot-missing] personal:layer.yaml: slot domain-glossary required by personal (requires_slots) is not filled by any layer"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn case_06_a_missing_slot_gives_a_stub_and_a_warning_without_strict_slots() {
    let found = merged_with(case_06(), LOOSE);
    assert_eq!(
        lines(&found),
        [
            "warning [slot-missing] personal:layer.yaml: slot domain-glossary required by personal (requires_slots) is not filled by any layer; emitting a stub skill"
        ]
    );
    let model = found.merged().expect("a warning does not fail the build");
    let stubs: Vec<_> = model
        .slot_stubs()
        .iter()
        .map(|(name, demand)| (name.as_str(), demand.clone()))
        .collect();
    assert_eq!(
        stubs,
        [(
            "domain-glossary",
            SlotDemand::Layer(crate::layer::LayerName::try_from("personal").expect("a layer name"))
        )]
    );
    assert!(model.skills().is_empty(), "a stub is not a skill");
}

#[test]
fn strict_slots_is_the_default_of_the_manifest() {
    assert!(
        merged(case_06())
            .diagnostics()
            .iter()
            .any(|found| found.to_string().starts_with("error [slot-missing]"))
    );
}

#[test]
fn a_slot_a_more_specific_layer_fills_is_not_missing() {
    let layers = vec![
        layer_from_yaml(CASE_06_PERSONAL_YAML, &[]),
        layer_from_yaml(
            CASE_06_REPO_YAML,
            &[(
                "skills/domain-glossary/SKILL.md",
                "---\nname: domain-glossary\ndescription: Terms\n---\n",
            )],
        ),
    ];
    let found = merged_with(layers, STRICT);
    assert_eq!(lines(&found), Vec::<String>::new());
    assert!(found.merged().expect("merged").slot_stubs().is_empty());
}

#[test]
fn a_command_fills_a_slot_a_layer_requires() {
    let layers = vec![
        layer_from_yaml(CASE_06_PERSONAL_YAML, &[]),
        layer_from_yaml(
            CASE_06_REPO_YAML,
            &[(
                "commands/domain-glossary.md",
                "---\ndescription: Terms\n---\n",
            )],
        ),
    ];
    assert_eq!(lines(&merged_with(layers, STRICT)), Vec::<String>::new());
}

const AGENT: &str =
    "---\nname: architect\ndescription: Plans\nskills: [adr, stack-conventions]\n---\nbody\n";

#[test]
fn a_skill_a_merged_agent_preloads_is_a_slot_that_must_exist() {
    let layers = vec![
        layer(
            "personal",
            "personal",
            &[
                ("agents/architect.md", AGENT),
                (
                    "skills/adr/SKILL.md",
                    "---\nname: adr\ndescription: Records\n---\n",
                ),
            ],
        ),
        layer("repo", "repo", &[]),
    ];
    assert_eq!(
        lines(&merged_with(layers.clone(), STRICT)),
        [
            "error [slot-missing] personal:agents/architect.md: slot stack-conventions required by agent architect is not filled by any layer"
        ]
    );
    let loose = merged_with(layers, LOOSE);
    assert_eq!(
        lines(&loose),
        [
            "warning [slot-missing] personal:agents/architect.md: slot stack-conventions required by agent architect is not filled by any layer; emitting a stub skill"
        ]
    );
    let model = loose.merged().expect("merged");
    assert_eq!(
        model
            .slot_stubs()
            .keys()
            .map(SkillName::as_str)
            .collect::<Vec<_>>(),
        ["stack-conventions"]
    );
}

#[test]
fn a_skill_an_overlay_adds_to_an_agent_is_a_slot_too() {
    let layers = vec![
        layer(
            "personal",
            "personal",
            &[(
                "agents/architect.md",
                "---\nname: architect\ndescription: Plans\n---\n",
            )],
        ),
        layer(
            "repo",
            "repo",
            &[(
                "agents/architect.overlay.md",
                "---\nextends: architect\nskills: +[domain-glossary]\n---\n",
            )],
        ),
    ];
    assert_eq!(
        lines(&merged_with(layers, STRICT)),
        [
            "error [slot-missing] personal:agents/architect.md: slot domain-glossary required by agent architect is not filled by any layer"
        ]
    );
}

#[test]
fn an_agent_that_preloads_a_command_is_an_error() {
    let layers = vec![layer(
        "personal",
        "personal",
        &[
            (
                "agents/architect.md",
                "---\nname: architect\ndescription: Plans\nskills: [release]\n---\n",
            ),
            ("commands/release.md", "---\ndescription: Ship\n---\n"),
        ],
    )];
    assert_eq!(
        lines(&merged(layers)),
        [
            "error [slot-command] personal:agents/architect.md: agent architect preloads release, which is a command; commands cannot be preloaded"
        ]
    );
}

#[test]
fn the_first_layer_to_require_a_slot_is_the_one_named() {
    let personal =
        "name: personal\nkind: personal\nvisibility: public\nrequires_slots: [domain-glossary]\n";
    let org = "name: acme\nkind: org\nvisibility: private\nrequires_slots: [domain-glossary]\n";
    let found = merged_with(
        vec![layer_from_yaml(personal, &[]), layer_from_yaml(org, &[])],
        STRICT,
    );
    assert_eq!(
        lines(&found),
        [
            "error [slot-missing] personal:layer.yaml: slot domain-glossary required by personal (requires_slots) is not filled by any layer"
        ]
    );
}

#[test]
fn a_requires_slots_entry_that_is_not_a_skill_name_is_an_error_and_never_a_stub() {
    let yaml =
        "name: personal\nkind: personal\nvisibility: public\nrequires_slots: [\"../escape\"]\n";
    let found = merged_with(vec![layer_from_yaml(yaml, &[])], LOOSE);
    assert_eq!(
        lines(&found),
        [
            "error [slot] personal:layer.yaml: requires_slots entry ../escape is not a kebab-case skill name"
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn merging_twice_gives_equal_models() {
    assert_eq!(merged(case_05()), merged(case_05()));
}

/// A layer's skills: each name with the extra files its directory holds.
type LayerSkills = BTreeMap<u8, std::collections::BTreeSet<u8>>;

fn skill_layers() -> impl Strategy<Value = Vec<LayerSkills>> {
    proptest::collection::vec(btree_map(0_u8..3, btree_set(0_u8..3, 0..3), 0..3), 1..5)
}

proptest! {
    /// The law of §7.3: for every name, the merged skill is the directory of the most specific
    /// layer that has it, with exactly that layer's files.
    #[test]
    fn the_most_specific_layer_that_has_a_name_wins_with_exactly_its_files(layers in skill_layers()) {
        let texts: Vec<Vec<(String, String)>> = layers
            .iter()
            .map(|skills| {
                skills
                    .iter()
                    .flat_map(|(name, extras)| {
                        std::iter::once((
                            format!("skills/s{name}/SKILL.md"),
                            format!("---\nname: s{name}\ndescription: d\n---\n"),
                        ))
                        .chain(extras.iter().map(move |extra| {
                            (format!("skills/s{name}/references/f{extra}.md"), "x".to_owned())
                        }))
                    })
                    .collect()
            })
            .collect();
        let stack: Vec<ResolvedLayer> = texts
            .iter()
            .enumerate()
            .map(|(index, files)| {
                let files: Vec<(&str, &str)> =
                    files.iter().map(|(path, text)| (path.as_str(), text.as_str())).collect();
                layer(&format!("l{index}"), "repo", &files)
            })
            .collect();
        let found = merged(stack);
        prop_assert_eq!(lines(&found), Vec::<String>::new());
        let model = found.merged().expect("merged");
        for name in 0_u8..3 {
            let winner = layers.iter().rev().find_map(|skills| skills.get(&name));
            let merged = model.skills().get(&skill_name(&format!("s{name}")));
            prop_assert_eq!(merged.is_some(), winner.is_some());
            if let (Some(extras), Some(traced)) = (winner, merged) {
                let mut expected: Vec<String> = std::iter::once("SKILL.md".to_owned())
                    .chain(extras.iter().map(|extra| format!("references/f{extra}.md")))
                    .collect();
                expected.sort();
                prop_assert_eq!(files_of(traced.value()), expected);
            }
        }
    }
}

// The spec names a skill by its folder and a command by its file in a header and in
// `alz explain`: `sourceRef(layer, dir)` and `sourceRef(layer, path)`.
#[test]
fn a_skill_is_named_by_its_folder_and_a_command_by_its_file() {
    use crate::merge::testing::layer_from_yaml;
    use crate::render::header::SourceRef;

    let yaml = "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n";
    let model = merged(vec![layer_from_yaml(
        yaml,
        &[
            ("layer.yaml", yaml),
            (
                "skills/arch/SKILL.md",
                "---\nname: arch\ndescription: d\n---\nbody\n",
            ),
            ("skills/arch/notes/more.md", "more\n"),
            ("commands/ship.md", "---\ndescription: d\n---\nbody\n"),
        ],
    )]);
    let model = model.merged().expect("no error was found");
    let source = |name: &str| {
        let skill = model
            .skills()
            .get(&SkillName::try_from(name).expect("a skill name"))
            .expect("the skill exists");
        SourceRef::of_contribution(&skill.provenance().contributions()[0])
            .expect("a source")
            .to_string()
    };
    assert_eq!(source("arch"), "personal@1.0.0:skills/arch");
    assert_eq!(source("ship"), "personal@1.0.0:commands/ship.md");
}
