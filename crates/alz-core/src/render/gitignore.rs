//! The managed block of `.gitignore` (spec §4.3 "The `.gitignore` block"): with
//! `commit_generated: false` the generated paths are listed between two marker lines, and the
//! text outside the block is kept byte for byte. Text in, text out; the writer reads and writes
//! the file.

use crate::render::tree::FileTree;

/// The first line of the block.
pub const BLOCK_START: &str = "# >>> agent-layerz generated (do not edit this block)";
/// The last line of the block.
pub const BLOCK_END: &str = "# <<< agent-layerz generated";

/// The start lines a block can open with: the current name and the earlier one.
const STARTS: [&str; 2] = [
    "# >>> agent-layerz generated",
    "# >>> agent-layers generated",
];
/// The end lines a block can close with.
const ENDS: [&str; 2] = [
    "# <<< agent-layerz generated",
    "# <<< agent-layers generated",
];

/// Whether the generated files are committed (§8.3 `commit_generated`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generated {
    /// `commit_generated: true`: no block.
    Committed,
    /// `commit_generated: false`: a block listing the generated paths.
    Ignored,
}

/// `current` with its managed block replaced by the block for `tree` when `generated` is
/// [`Generated::Ignored`], or removed when it is [`Generated::Committed`].
///
/// The block, current or earlier, is the first one found: from a start line to the next end line
/// and the line feed after it. A line feed is added before a new block when the text before it
/// does not end with one. Every other byte stays.
#[must_use]
pub fn synced(current: &str, tree: &FileTree, generated: Generated) -> String {
    let without = without_block(current);
    match generated {
        Generated::Committed => without,
        Generated::Ignored => {
            let separator = if without.is_empty() || without.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            let lines = tree
                .paths()
                .map(|path| format!("/{path}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{without}{separator}{BLOCK_START}\n{lines}\n{BLOCK_END}\n")
        }
    }
}

/// `text` without its first block.
fn without_block(text: &str) -> String {
    block_span(text).map_or_else(
        || text.to_owned(),
        |(from, to)| {
            format!(
                "{}{}",
                text.get(..from).unwrap_or_default(),
                text.get(to..).unwrap_or_default()
            )
        },
    )
}

/// Where the first block lies: the byte of its start marker and the byte after its end line,
/// line feed included. A start with no end after its own line is skipped.
fn block_span(text: &str) -> Option<(usize, usize)> {
    markers(text, 0, &STARTS).find_map(|(from, _)| {
        let line_end = text
            .get(from..)?
            .find('\n')?
            .checked_add(from)?
            .checked_add(1)?;
        let (at, marker) = markers(text, line_end, &ENDS).next()?;
        let after = at.checked_add(marker.len())?;
        let to = if text.get(after..)?.starts_with('\n') {
            after.checked_add(1)?
        } else {
            after
        };
        Some((from, to))
    })
}

/// The offsets at or after `from` where one of `markers` starts, with the marker, in order.
fn markers<'a>(
    text: &'a str,
    from: usize,
    markers: &'a [&'a str],
) -> impl Iterator<Item = (usize, &'a str)> + 'a {
    text.char_indices()
        .map(|(at, _)| at)
        .skip_while(move |at| *at < from)
        .filter_map(move |at| {
            let rest = text.get(at..)?;
            markers
                .iter()
                .find(|marker| rest.starts_with(**marker))
                .map(|marker| (at, *marker))
        })
}

#[cfg(test)]
mod tests;
