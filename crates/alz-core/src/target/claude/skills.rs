//! `.claude/skills/<name>/` (spec §8.1, target-facts §5): every file of a winning skill folder,
//! a command as a skill only the user can invoke, and the stub of a slot no layer filled.

use alz_adapter::{Command, Model, Skill, Source};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
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
pub(super) fn render(model: &Model) -> Output {
    let skills = model.skills.iter().map(directory_files);
    let commands = model.commands.iter().map(command_file);
    let stubs = model.slot_stubs.iter().map(|stub| stub_file(&stub.name));
    skills
        .chain(commands)
        .chain(stubs)
        .fold(Output::default(), Output::then)
}

fn directory_files(skill: &Skill) -> Output {
    sources(&skill.sources).map_or_else(
        |problems| Output {
            files: Vec::new(),
            diagnostics: problems,
        },
        |header| {
            skill
                .files
                .iter()
                .map(|skill_file| {
                    let path =
                        output_path(&format!("{SKILLS_DIR}/{}/{}", skill.name, skill_file.path));
                    if skill_file.path == SKILL_FILE {
                        skill_markdown(&skill_file.content, &header, path, &skill.sources)
                    } else {
                        let mode = if skill_file.executable {
                            FileMode::Executable
                        } else {
                            FileMode::Regular
                        };
                        Output::one(
                            path,
                            GeneratedFile {
                                bytes: skill_file.content.clone(),
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
    header: &Sources,
    path: Result<crate::render::tree::OutputPath, Diagnostic>,
    named: &[Source],
) -> Output {
    let made = std::str::from_utf8(bytes)
        .ok()
        .and_then(|markdown| insert_frontmatter_header(markdown, header).ok());
    match made {
        Some(markdown) => Output::one(path, GeneratedFile::text(&markdown), Diagnostics::new()),
        None => Output {
            files: Vec::new(),
            diagnostics: Diagnostics::new().with(Diagnostic::error(
                Code::Skill,
                place(named),
                Message::new("SKILL.md has no frontmatter block to put the header in"),
            )),
        },
    }
}

fn command_file(command: &Command) -> Output {
    let fields = [
        Some(("name", text(&command.name))),
        Some(("description", text(&command.description))),
        command
            .argument_hint
            .as_deref()
            .map(|hint| ("argument-hint", text(hint))),
        Some(("disable-model-invocation", EmitValue::Bool(true))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    file(
        &format!("{SKILLS_DIR}/{}/{SKILL_FILE}", command.name),
        sources(&command.sources),
        fields,
        &command.body,
        Diagnostics::new(),
    )
}

fn stub_file(name: &str) -> Output {
    let fields = vec![
        ("name".to_owned(), text(name)),
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
