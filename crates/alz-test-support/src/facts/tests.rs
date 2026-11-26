//! Spec §8 **What an adapter may rely on** and §8.0 **Descriptor.** (its facts file), proved on a
//! fake adapter: the harness lists the keys of structured output, an unrecorded key fails, a
//! recorded key passes and a stale exemption fails.

use std::collections::BTreeMap;

use alz_adapter::{
    Adapter, Bytes, Content, Context, Descriptor, Frontmatter, JsonObject, JsonValue,
    MODEL_VERSION, Markdown, Model, Output, OutputFile, TargetId, TomlValue, ToolVersion,
    ToolVersionRange,
};

use super::{
    Facts, FactsError, KeyPath, Part, Subject, SubjectMap, check, is_unverified, written_files,
};
use crate::ScratchDir;

const SETTINGS: Subject = Subject {
    sections: &["## 1. "],
    row_holds: "settings",
};
const FIELDS: Subject = Subject {
    sections: &["## 2. "],
    row_holds: "",
};
const PLACES: Subject = Subject {
    sections: &["## 3. "],
    row_holds: ".fake/",
};

/// The kinds of output of the fake adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Setting,
    Nested,
    Field,
    Place,
}

/// A fake adapter that writes one JSON file, one Markdown file and one file of bytes.
struct Fake {
    descriptor: Descriptor,
    setting: &'static str,
}

impl Fake {
    fn writing(setting: &'static str) -> Self {
        Self {
            descriptor: Descriptor::new(
                TargetId::try_from("fake").expect("a target id"),
                MODEL_VERSION,
                ToolVersionRange::new(ToolVersion::new(1, 0, 0), ToolVersion::new(2, 0, 0)),
                "docs/fake-facts.md",
            ),
            setting,
        }
    }
}

fn object(entries: Vec<(&str, JsonValue)>) -> JsonValue {
    JsonValue::Object(
        JsonObject::from_entries(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
        .expect("distinct keys"),
    )
}

impl Adapter for Fake {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    fn render(&self, _model: &Model, _context: &Context) -> Output {
        let settings = object(vec![
            (self.setting, JsonValue::Bool(true)),
            (
                "nested",
                object(vec![(
                    "groups",
                    JsonValue::Array(vec![object(vec![("matcher", JsonValue::Text("x".into()))])]),
                )]),
            ),
        ]);
        let frontmatter = JsonObject::from_entries(vec![(
            "name".to_owned(),
            JsonValue::Text("agent".to_owned()),
        )])
        .expect("distinct keys");
        Output::new(
            vec![
                OutputFile::new(".fake/settings.json", Content::Json(settings)),
                OutputFile::new(
                    ".fake/agent.md",
                    Content::Markdown(
                        Markdown::new("Body.").with_frontmatter(Frontmatter::Mapping(frontmatter)),
                    ),
                ),
                OutputFile::new(
                    ".fake/run.sh",
                    Content::Bytes(Bytes::new(b"#!/bin/sh\n".to_vec())),
                ),
            ],
            Vec::new(),
            alz_adapter::Coverage::default(),
        )
    }
}

impl SubjectMap for Fake {
    type Kind = Kind;

    fn location(&self, file: &str) -> Option<(Kind, Subject)> {
        file.starts_with(".fake/").then_some((Kind::Place, PLACES))
    }

    fn key(&self, file: &str, path: &KeyPath) -> Option<(Kind, Subject)> {
        match (file, path.parts().as_slice()) {
            (".fake/settings.json", [Part::Key(_)]) => Some((Kind::Setting, SETTINGS)),
            (
                ".fake/settings.json",
                [Part::Key("nested"), Part::Key(_), Part::Items, Part::Key(_)],
            ) => Some((Kind::Nested, SETTINGS)),
            (".fake/agent.md", [Part::Key(_)]) => Some((Kind::Field, FIELDS)),
            _ => None,
        }
    }
}

const FACTS: &str = "## 1. Settings\n\n| Item | Fact | Status |\n|---|---|---|\n| settings | `known`, `nested`, `matcher` | DOCS |\n| settings | `guess` | UNVERIFIED |\n\n## 2. Fields\n\n| Item | Fact | Status |\n|---|---|---|\n| fields | `name` | DOCS |\n\n## 3. Places\n\n| Item | Fact | Status |\n|---|---|---|\n| place | `.fake/` holds the files | DOCS |\n";

fn output_of(fake: &Fake) -> Output {
    let target = fake.descriptor().target.clone();
    fake.render(&Model::new(target), &Context::default())
}

fn kinds(found: &std::collections::BTreeSet<(Kind, String)>) -> Vec<(Kind, &str)> {
    found
        .iter()
        .map(|(kind, key)| (*kind, key.as_str()))
        .collect()
}

#[test]
fn the_keys_of_json_frontmatter_toml_and_kept_text_are_listed_as_key_paths() {
    let toml = TomlValue::Table(BTreeMap::from([(
        "model".to_owned(),
        TomlValue::Table(BTreeMap::from([(
            "providers".to_owned(),
            TomlValue::Array(vec![TomlValue::Table(BTreeMap::from([(
                "id".to_owned(),
                TomlValue::Text("a".into()),
            )]))]),
        )])),
    )]));
    let kept = Markdown::new("Body.").with_frontmatter(Frontmatter::Kept(
        "name: a\n# comment\ndescription: >\n  folded: not a key\n\"argument-hint\": x\nlist:\n  - item: 1\n"
            .to_owned(),
    ));
    let output = Output::new(
        vec![
            OutputFile::new("a.toml", Content::Toml(toml)),
            OutputFile::new("b.md", Content::Markdown(kept)),
            OutputFile::new("c.sh", Content::Bytes(Bytes::new(Vec::new()))),
        ],
        Vec::new(),
        alz_adapter::Coverage::default(),
    );
    let listed: Vec<(String, Vec<String>)> = written_files(&output)
        .into_iter()
        .map(|file| {
            (
                file.file,
                file.keys.iter().map(ToString::to_string).collect(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (
                "a.toml".to_owned(),
                vec![
                    "model".to_owned(),
                    "model.providers".to_owned(),
                    "model.providers[].id".to_owned()
                ]
            ),
            (
                "b.md".to_owned(),
                vec![
                    "argument-hint".to_owned(),
                    "description".to_owned(),
                    "list".to_owned(),
                    "name".to_owned()
                ]
            ),
            ("c.sh".to_owned(), Vec::new()),
        ]
    );
    let fake = written_files(&output_of(&Fake::writing("known")));
    let settings: Vec<String> = fake
        .first()
        .map_or_default(|file| file.keys.iter().map(ToString::to_string).collect());
    assert_eq!(
        settings,
        [
            "known",
            "nested",
            "nested.groups",
            "nested.groups[].matcher"
        ]
    );
}

#[test]
fn a_key_that_no_verified_row_records_fails() {
    let fake = Fake::writing("bogus");
    let found = check(&Facts::from_text(FACTS), &fake, &output_of(&fake), &[]);
    assert_eq!(kinds(&found.unrecorded), [(Kind::Setting, "bogus")]);
    assert!(found.unplaced.is_empty() && !found.is_clean());
}

#[test]
fn a_key_that_only_an_unverified_row_or_another_section_records_fails() {
    let fake = Fake::writing("guess");
    let found = check(&Facts::from_text(FACTS), &fake, &output_of(&fake), &[]);
    assert_eq!(kinds(&found.unrecorded), [(Kind::Setting, "guess")]);
    let elsewhere = Fake::writing("name");
    let found = check(
        &Facts::from_text(FACTS),
        &elsewhere,
        &output_of(&elsewhere),
        &[],
    );
    assert_eq!(kinds(&found.unrecorded), [(Kind::Setting, "name")]);
}

#[test]
fn a_key_that_a_verified_row_records_passes() {
    let fake = Fake::writing("known");
    let found = check(&Facts::from_text(FACTS), &fake, &output_of(&fake), &[]);
    assert!(found.is_clean(), "{found}");
}

#[test]
fn a_location_that_no_verified_row_records_fails() {
    let fake = Fake::writing("known");
    let facts = FACTS.replace(
        "| place | `.fake/` holds the files | DOCS |",
        "| place | `.fake/` | UNVERIFIED |",
    );
    let found = check(&Facts::from_text(facts), &fake, &output_of(&fake), &[]);
    assert_eq!(kinds(&found.unrecorded), [(Kind::Place, "")]);
}

#[test]
fn a_file_the_subject_map_has_no_place_for_fails() {
    struct Nowhere(Fake);
    impl SubjectMap for Nowhere {
        type Kind = Kind;
        fn location(&self, _file: &str) -> Option<(Kind, Subject)> {
            None
        }
        fn key(&self, file: &str, path: &KeyPath) -> Option<(Kind, Subject)> {
            self.0.key(file, path)
        }
    }
    let map = Nowhere(Fake::writing("known"));
    let found = check(&Facts::from_text(FACTS), &map, &output_of(&map.0), &[]);
    assert_eq!(found.unplaced.len(), 3);
}

#[test]
fn an_exemption_for_a_key_that_is_still_unrecorded_passes() {
    let fake = Fake::writing("bogus");
    let found = check(
        &Facts::from_text(FACTS),
        &fake,
        &output_of(&fake),
        &[(Kind::Setting, "bogus")],
    );
    assert!(found.is_clean(), "{found}");
}

#[test]
fn a_stale_exemption_fails_when_no_output_uses_it_any_more() {
    let fake = Fake::writing("known");
    let found = check(
        &Facts::from_text(FACTS),
        &fake,
        &output_of(&fake),
        &[(Kind::Setting, "gone")],
    );
    assert_eq!(kinds(&found.unused_exemptions), [(Kind::Setting, "gone")]);
    assert!(found.recorded_exemptions.is_empty() && !found.is_clean());
}

#[test]
fn a_stale_exemption_fails_when_the_facts_record_the_key_now() {
    let fake = Fake::writing("known");
    let found = check(
        &Facts::from_text(FACTS),
        &fake,
        &output_of(&fake),
        &[(Kind::Setting, "known")],
    );
    assert_eq!(
        kinds(&found.recorded_exemptions),
        [(Kind::Setting, "known")]
    );
    assert!(found.unused_exemptions.is_empty() && !found.is_clean());
}

#[test]
fn a_row_is_unverified_only_by_the_status_cell() {
    assert!(is_unverified("| a | `b` | UNVERIFIED |"));
    assert!(!is_unverified("| UNVERIFIED | `b` | DOCS |"));
}

#[test]
fn the_facts_are_read_from_the_file_the_descriptor_names_below_the_repository() {
    let repository = ScratchDir::new().expect("a scratch directory");
    std::fs::create_dir(repository.path().join("docs")).expect("a folder");
    std::fs::write(repository.path().join("docs/fake-facts.md"), FACTS).expect("a file");
    let fake = Fake::writing("known");
    let facts = Facts::of(fake.descriptor(), repository.path()).expect("the facts");
    assert!(check(&facts, &fake, &output_of(&fake), &[]).is_clean());

    let mut outside = Fake::writing("known");
    outside.descriptor.facts_file = "../facts.md".to_owned();
    assert!(matches!(
        Facts::of(outside.descriptor(), repository.path()),
        Err(FactsError::NotBelowRepository(_))
    ));
    let mut missing = Fake::writing("known");
    missing.descriptor.facts_file = "docs/none.md".to_owned();
    assert!(matches!(
        Facts::of(missing.descriptor(), repository.path()),
        Err(FactsError::Read { .. })
    ));
}
