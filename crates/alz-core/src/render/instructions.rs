//! `AGENTS.md` and `CLAUDE.md` (spec §7.1, §7.12, §8.1): the composed instructions of the stack
//! and the thin Claude file that imports them, each with its generated header (§4.3).
//!
//! `AGENTS.md` is the header, then one `## <Kind>: <name>` heading for each run of blocks of one
//! layer with the blocks under it, then the Commits section of the winning commit identity
//! (§7.12). The text is composed from the merged model alone, and the same model gives the same
//! bytes. Both budgets count the whole text, the Commits section included.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::identity::CommitIdentity;
use crate::layer::LayerName;
use crate::merge::{InstructionBlock, Merged, MergedClaudeExtra, Traced};
use crate::render::header::{SourceRef, Sources, html_header};
use crate::render::sources::refs_of;
use crate::render::tree::OutputPath;
use crate::stack::Budgets;

/// The path of the composed instructions.
const AGENTS_MD: &str = "AGENTS.md";
/// The path of the Claude file that imports them.
const CLAUDE_MD: &str = "CLAUDE.md";
/// The heading of the Claude-only text in `CLAUDE.md`.
const CLAUDE_SPECIFIC: &str = "## Claude-specific";
/// The separator of the parts of a composed file.
const PART_SEPARATOR: &str = "\n\n";

/// The path of `AGENTS.md`.
///
/// # Panics
///
/// Never: the literal is a safe relative path, which the tests show.
#[must_use]
pub fn agents_md_path() -> OutputPath {
    OutputPath::try_from(AGENTS_MD).expect("AGENTS.md is a safe relative path")
}

/// The path of `CLAUDE.md`.
///
/// # Panics
///
/// Never: the literal is a safe relative path, which the tests show.
#[must_use]
pub fn claude_md_path() -> OutputPath {
    OutputPath::try_from(CLAUDE_MD).expect("CLAUDE.md is a safe relative path")
}

/// The composed `AGENTS.md`: the header, the blocks of every layer and the Commits section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsMd(String);

impl AgentsMd {
    /// Composes `AGENTS.md` from the blocks and the commit identity of `merged`.
    ///
    /// # Errors
    ///
    /// An error `layer` for each source of the header that holds a control or format character
    /// or `-->`, which would end the header's comment (§4.3).
    pub fn compose(merged: &Merged) -> Result<Self, Diagnostics> {
        let commits = merged
            .commit_identity()
            .declared()
            .origin()
            .zip(merged.commit_identity().declared().winner());
        let origins = merged
            .instructions()
            .iter()
            .map(|block| block.value().origin())
            .chain(commits.map(|(origin, _)| origin));
        refs_of(origins).map(|refs| {
            let mut refs = refs.into_iter();
            let sources = refs.next().map_or_else(
                || Sources::single(SourceRef::NoInstructions),
                |first| Sources::new(first, refs),
            );
            let header = html_header(&sources);
            let blocks = blocks_text(merged.instructions());
            let section = commits.map(|(_, (layer, identity))| commits_section(layer, identity));
            Self(finish(std::iter::once(header).chain(blocks).chain(section)))
        })
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }

    /// The text, handed over.
    #[must_use]
    pub fn into_text(self) -> String {
        self.0
    }

    /// The error `budget` for each limit the text is over (§7.1): lines are the line feeds it
    /// holds, bytes its UTF-8 length.
    #[must_use]
    pub fn budget_problems(&self, budgets: &Budgets) -> Diagnostics {
        let lines = count(self.0.matches('\n').count());
        let bytes = count(self.0.len());
        let over_lines = budgets
            .instructions_max_lines()
            .filter(|max| lines > *max)
            .map(|max| {
                Message::new("AGENTS.md has ")
                    .then(lines)
                    .then(" lines; instructions_max_lines is ")
                    .then(max)
                    .then(". Move long standards into skills")
            });
        let over_bytes = Some(budgets.instructions_max_bytes())
            .filter(|max| bytes > *max)
            .map(|max| {
                Message::new("AGENTS.md is ")
                    .then(bytes)
                    .then(" bytes; instructions_max_bytes is ")
                    .then(max)
            });
        over_lines
            .into_iter()
            .chain(over_bytes)
            .map(|message| Diagnostic::error(Code::Budget, None, message))
            .collect()
    }
}

/// The thin `CLAUDE.md`: the header, the import of `AGENTS.md`, and the `## Claude-specific`
/// block with the text of each layer's `targets/claude/CLAUDE.extra.md` in stack order (§7.1).
///
/// # Errors
///
/// An error `layer` for each source of the header that holds a control or format character or
/// `-->` (§4.3).
pub fn claude_md(merged: &Merged) -> Result<String, Diagnostics> {
    let extras = merged.claude_extras();
    refs_of(extras.iter().map(MergedClaudeExtra::origin)).map(|refs| {
        let sources = Sources::new(SourceRef::Output(agents_md_path()), refs);
        let specific = (!extras.is_empty())
            .then(|| CLAUDE_SPECIFIC.to_owned())
            .into_iter()
            .chain(extras.iter().map(|extra| extra.body().to_owned()));
        finish(
            [html_header(&sources), format!("@{AGENTS_MD}")]
                .into_iter()
                .chain(specific),
        )
    })
}

/// The parts joined by blank lines, ending with one line feed.
fn finish(parts: impl IntoIterator<Item = String>) -> String {
    format!(
        "{}\n",
        parts.into_iter().collect::<Vec<_>>().join(PART_SEPARATOR)
    )
}

/// A count as the budgets are: a saturating `u64`.
fn count(number: usize) -> u64 {
    u64::try_from(number).unwrap_or(u64::MAX)
}

/// The text of the blocks: the heading of a layer before the first block of each run of its
/// blocks, then the block's text.
fn blocks_text(blocks: &[Traced<InstructionBlock>]) -> Vec<String> {
    blocks
        .iter()
        .map(Traced::value)
        .fold((Vec::new(), None), |(parts, current), block| {
            let heading = block.heading();
            let parts = parts
                .into_iter()
                .chain((current != Some(heading)).then(|| heading.to_string()))
                .chain([block.body().to_owned()])
                .collect();
            (parts, Some(heading))
        })
        .0
}

/// The Commits section (§7.12) for the identity `layer` declares.
fn commits_section(layer: &LayerName, identity: &CommitIdentity) -> String {
    [
        "## Commits".to_owned(),
        String::new(),
        format!(
            "- Author and committer of every commit made in a local clone: `{} <{}>` (declared by layer `{layer}`). This repository's local git config sets it; never change `user.name` or `user.email`, and never commit locally under a CI, bot or shared identity.",
            identity.name().as_str(),
            identity.email().as_str()
        ),
        "- A commit made by an agent ends with a `Co-Authored-By:` trailer that names the agent. Use the trailer your tool adds; if it adds none, write one.".to_owned(),
        "- If `git var GIT_AUTHOR_IDENT` shows another identity, do not commit; report it.".to_owned(),
        "- Bots that commit on the hosting service, such as release or CI bots, are exempt from this rule.".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests;
