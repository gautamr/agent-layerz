//! Skills, commands and slot stubs (spec §7.3, §7.4).

use alz_adapter::{Command, Skill, SkillFile, SlotStub};

use crate::item::{self, SkillDirectory};
use crate::merge::{Contribution, Merged, Traced};

use super::sources::of_origin;

/// The definition that stands: the last contribution, since a skill replaces the whole definition
/// before it, so the earlier ones are overridden.
fn winning(traced: &Traced<item::Skill>) -> Vec<alz_adapter::Source> {
    traced
        .provenance()
        .contributions()
        .last()
        .map(Contribution::origin)
        .map(of_origin)
        .into_iter()
        .collect()
}

/// The skills: the winning folder of each, every file kept as the layer has it.
pub(super) fn skills(merged: &Merged) -> Vec<Skill> {
    merged
        .skills()
        .iter()
        .filter_map(|(_, traced)| match traced.value() {
            item::Skill::Directory(directory) => Some(skill(directory, traced)),
            item::Skill::Command(_) => None,
        })
        .collect()
}

fn skill(directory: &SkillDirectory, traced: &Traced<item::Skill>) -> Skill {
    let mut skill = Skill::new(
        directory.name().as_str(),
        directory
            .files()
            .iter()
            .map(|file| {
                SkillFile::new(
                    file.within().collect::<Vec<_>>().join("/"),
                    file.bytes().to_vec(),
                    file.executable(),
                )
            })
            .collect(),
    );
    skill.sources = winning(traced);
    skill
}

/// The commands.
pub(super) fn commands(merged: &Merged) -> Vec<Command> {
    merged
        .skills()
        .iter()
        .filter_map(|(_, traced)| match traced.value() {
            item::Skill::Command(command) => Some(one_command(command, traced)),
            item::Skill::Directory(_) => None,
        })
        .collect()
}

fn one_command(command: &item::Command, traced: &Traced<item::Skill>) -> Command {
    let mut made = Command::new(
        command.name().as_str(),
        command.description().as_str(),
        command.body(),
    );
    made.argument_hint = command.argument_hint().map(|hint| hint.as_str().to_owned());
    made.sources = winning(traced);
    made
}

/// The slots that no layer defines.
pub(super) fn slot_stubs(merged: &Merged) -> Vec<SlotStub> {
    merged
        .slot_stubs()
        .keys()
        .map(|name| SlotStub::new(name.as_str()))
        .collect()
}
