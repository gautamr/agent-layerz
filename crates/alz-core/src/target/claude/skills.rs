//! `.claude/skills/<name>/` (spec §8.1, target-facts §5): every file of a winning skill folder,
//! a command as a skill only the user can invoke, and the stub of a slot no layer filled.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::item::{Command, Skill, SkillDirectory, SkillName};
use crate::merge::{Merged, Origin, Traced};
use crate::render::header::{SourceRef, Sources, insert_frontmatter_header};
use crate::render::tree::{FileMode, GeneratedFile};
use crate::render::yaml::EmitValue;

use super::frontmatter::{file, place, sources, text};
use super::{Output, output_path};

/// The folder of the skill folders.
const SKILLS_DIR: &str = ".claude/skills";
/// The skill file that carries the frontmatter.
const SKILL_FILE: &str = "SKILL.md";

/// The skills, commands and slot stubs as files.
pub(super) fn render(merged: &Merged) -> Output {
    let skills = merged
        .skills()
        .iter()
        .map(|(_, traced)| match traced.value() {
            Skill::Directory(directory) => directory_files(directory, traced),
            Skill::Command(command) => command_file(command, traced),
        });
    let stubs = merged.slot_stubs().keys().map(stub_file);
    skills.chain(stubs).fold(Output::default(), Output::then)
}

/// The origin that defined the skill that stands: its last contribution, since a skill replaces
/// the whole definition before it.
fn winning(traced: &Traced<Skill>) -> impl Iterator<Item = &Origin> + '_ {
    traced
        .provenance()
        .contributions()
        .last()
        .map(crate::merge::Contribution::origin)
        .into_iter()
}

fn directory_files(directory: &SkillDirectory, traced: &Traced<Skill>) -> Output {
    sources(winning(traced)).map_or_else(
        |problems| Output {
            files: Vec::new(),
            diagnostics: problems,
        },
        |sources| {
            directory
                .files()
                .iter()
                .map(|skill_file| {
                    let within = skill_file.within().collect::<Vec<_>>().join("/");
                    let path = output_path(&format!("{SKILLS_DIR}/{}/{within}", directory.name()));
                    if within == SKILL_FILE {
                        skill_markdown(
                            skill_file.bytes(),
                            &sources,
                            path,
                            place(traced.provenance()),
                        )
                    } else {
                        let mode = if skill_file.executable() {
                            FileMode::Executable
                        } else {
                            FileMode::Regular
                        };
                        Output::one(
                            path,
                            GeneratedFile {
                                bytes: skill_file.bytes().to_vec(),
                                mode,
                            },
                            Diagnostics::new(),
                        )
                    }
                })
                .fold(Output::default(), Output::then)
        },
    )
}

/// `SKILL.md` with the header as the first line of its frontmatter.
fn skill_markdown(
    bytes: &[u8],
    sources: &Sources,
    path: Result<crate::render::tree::OutputPath, Diagnostic>,
    at: Option<crate::diagnostic::Where>,
) -> Output {
    let made = std::str::from_utf8(bytes)
        .ok()
        .and_then(|markdown| insert_frontmatter_header(markdown, sources).ok());
    match made {
        Some(markdown) => Output::one(path, GeneratedFile::text(&markdown), Diagnostics::new()),
        None => Output {
            files: Vec::new(),
            diagnostics: Diagnostics::new().with(Diagnostic::error(
                Code::Skill,
                at,
                Message::new("SKILL.md has no frontmatter block to put the header in"),
            )),
        },
    }
}

fn command_file(command: &Command, traced: &Traced<Skill>) -> Output {
    let fields = [
        Some(("name", text(command.name().as_str()))),
        Some(("description", text(command.description().as_str()))),
        command
            .argument_hint()
            .map(|hint| ("argument-hint", text(hint.as_str()))),
        Some(("disable-model-invocation", EmitValue::Bool(true))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    file(
        &format!("{SKILLS_DIR}/{}/{SKILL_FILE}", command.name()),
        sources(winning(traced)),
        fields,
        command.body(),
        Diagnostics::new(),
    )
}

fn stub_file(name: &SkillName) -> Output {
    let fields = vec![
        ("name".to_owned(), text(name.as_str())),
        (
            "description".to_owned(),
            text(&format!("Slot stub: {name} is not defined for this repo")),
        ),
    ];
    file(
        &format!("{SKILLS_DIR}/{name}/{SKILL_FILE}"),
        Ok(Sources::single(SourceRef::SlotStub)),
        fields,
        &format!(
            "The `{name}` skill is not defined for this repo. Do not assume its conventions; ask the user when they matter."
        ),
        Diagnostics::new(),
    )
}
