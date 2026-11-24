//! `CLAUDE.md` (spec §4.3): the generated one imports `AGENTS.md` itself, so the adopted copy drops
//! that import and every paragraph `AGENTS.md` already holds (tools such as Backlog.md write the
//! same block to both).

use std::collections::BTreeSet;

use crate::diagnostic::Message;

use super::draft::Draft;

/// Where the adopted text goes, from the repo layer.
const CLAUDE_EXTRA: &str = "targets/claude/CLAUDE.extra.md";

/// Plans the adoption of the `CLAUDE.md` whose bytes are `bytes`.
pub(super) fn adopt(bytes: &[u8], draft: &mut Draft<'_>) {
    let Ok(text) = std::str::from_utf8(bytes) else {
        draft.problem("CLAUDE.md", Message::new("is not valid UTF-8 text"));
        return;
    };
    let known: BTreeSet<String> = draft
        .ctx
        .agents_md
        .map_or_else(Vec::new, |bytes| {
            paragraphs(&String::from_utf8_lossy(bytes))
        })
        .into_iter()
        .collect();
    let extra = paragraphs(text)
        .into_iter()
        .map(|paragraph| {
            paragraph
                .lines()
                .filter(|line| line.trim() != "@AGENTS.md")
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_owned()
        })
        .filter(|paragraph| !paragraph.is_empty() && !known.contains(paragraph))
        .collect::<Vec<_>>()
        .join("\n\n");
    if extra.is_empty() {
        draft.converted("CLAUDE.md", &[]);
    } else {
        draft.write_text("CLAUDE.md", CLAUDE_EXTRA, format!("{extra}\n"));
        draft.converted("CLAUDE.md", &[CLAUDE_EXTRA.to_owned()]);
    }
}

/// The paragraphs of a text: runs of lines that are not blank, each trimmed.
pub(super) fn paragraphs(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .split('\n')
        .fold(vec![Vec::<&str>::new()], |mut groups, line| {
            if line.trim().is_empty() {
                groups.push(Vec::new());
            } else if let Some(last) = groups.last_mut() {
                last.push(line);
            }
            groups
        })
        .into_iter()
        .map(|lines| lines.join("\n").trim().to_owned())
        .collect()
}
