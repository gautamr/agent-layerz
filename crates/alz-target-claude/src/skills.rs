//! `.claude/skills/<name>/` (spec §8.1, target-facts §5): every file of a winning skill folder,
//! a command as a skill only the user can invoke, and the stub of a slot no layer filled.

use alz_adapter::{
    Bytes, Command, Content, Frontmatter, HeaderSource, JsonValue, Level, Markdown, Model, Skill,
    SkillFile,
};

use super::frontmatter::{file, header_sources, text};
use super::{Made, found, place};

/// The folder of the skill folders.
const SKILLS_DIR: &str = ".claude/skills";
/// The skill file that carries the frontmatter.
const SKILL_FILE: &str = "SKILL.md";
/// The line that opens and closes a frontmatter block.
const FENCE: &str = "---";

/// The skills, commands and slot stubs as files.
pub(super) fn render(model: &Model) -> Made {
    let skills = model.skills.iter().map(directory_files);
    let commands = model.commands.iter().map(command_file);
    let stubs = model.slot_stubs.iter().map(|stub| stub_file(&stub.name));
    skills
        .chain(commands)
        .chain(stubs)
        .fold(Made::default(), Made::then)
}

fn directory_files(skill: &Skill) -> Made {
    skill
        .files
        .iter()
        .map(|skill_file| {
            let path = format!("{SKILLS_DIR}/{}/{}", skill.name, skill_file.path);
            if skill_file.path == SKILL_FILE {
                skill_markdown(skill_file, path, skill)
            } else {
                Made::file(
                    path,
                    Content::Bytes(
                        Bytes::new(skill_file.content.clone()).executable(skill_file.executable),
                    ),
                )
            }
        })
        .fold(Made::default(), Made::then)
}

/// `SKILL.md` with its frontmatter kept byte for byte, in line feeds, and the body after it. The
/// core puts the header as the first line of the frontmatter.
fn skill_markdown(skill_file: &SkillFile, path: String, skill: &Skill) -> Made {
    match kept_parts(&skill_file.content) {
        Some((frontmatter, body)) => Made::file(
            path,
            Content::Markdown(
                Markdown::new(body)
                    .with_frontmatter(Frontmatter::Kept(frontmatter))
                    .with_header_sources(header_sources(&skill.sources)),
            ),
        ),
        None => Made::problem(found(
            Level::Error,
            "claude-agent",
            place(&skill.sources),
            "SKILL.md has no frontmatter block to put the header in".to_owned(),
        )),
    }
}

/// The text between the opening and the closing `---` line, with its line feeds, and the text
/// after the closing `---`, which starts with its line feed when the file has one. Line endings are read as line feeds throughout, as the spec
/// reads them. `None` when the text opens no block or never closes it.
fn kept_parts(bytes: &[u8]) -> Option<(String, String)> {
    let text = std::str::from_utf8(bytes).ok()?.replace("\r\n", "\n");
    let rest = text.strip_prefix("---\n")?;
    let closing = rest
        .split_inclusive('\n')
        .try_fold(0_usize, |offset, line| {
            if line.strip_suffix('\n').unwrap_or(line) == FENCE {
                Err(offset)
            } else {
                Ok(offset + line.len())
            }
        })
        .err()?;
    let (before, from_fence) = rest.split_at(closing);
    let after = from_fence.strip_prefix(FENCE).unwrap_or_default();
    Some((before.to_owned(), after.to_owned()))
}

fn command_file(command: &Command) -> Made {
    let fields = [
        Some(("name", text(&command.name))),
        Some(("description", text(&command.description))),
        command
            .argument_hint
            .as_deref()
            .map(|hint| ("argument-hint", text(hint))),
        Some(("disable-model-invocation", JsonValue::Bool(true))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    file(
        &format!("{SKILLS_DIR}/{}/{SKILL_FILE}", command.name),
        header_sources(&command.sources),
        fields,
        &command.body,
    )
}

fn stub_file(name: &str) -> Made {
    let fields = vec![
        ("name".to_owned(), text(name)),
        (
            "description".to_owned(),
            text(&format!("Slot stub: {name} is not defined for this repo")),
        ),
    ];
    file(
        &format!("{SKILLS_DIR}/{name}/{SKILL_FILE}"),
        vec![HeaderSource::SlotStub],
        fields,
        &format!(
            "The `{name}` skill is not defined for this repo. Do not assume its conventions; ask the user when they matter."
        ),
    )
}
