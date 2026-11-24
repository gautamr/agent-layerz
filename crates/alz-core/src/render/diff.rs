//! The unified diff of two files that `alz diff` prints (spec §10.3 "alz diff"): a shortest edit
//! script over lines (Myers), hunks with three lines of context, and `Binary file <path> changes`
//! for content that is no text.
//!
//! The search is bounded (see [`COST_CAP`]), so a crafted file cannot make it quadratic.
//!
//! The diff is computed over the raw lines. Untrusted text is stripped (class 1) only when a patch
//! is printed, so what is compared is never altered.

mod myers;

use crate::untrusted_text::Stripped;
use myers::{Edit, script};

/// Lines of context before and after each change.
const CONTEXT: usize = 3;

/// The most work the edit search may do: about `(old lines + new lines) * edits` steps, after the
/// lines both texts share at the start and at the end are set aside. Past it, the lines between
/// those shared ends are removed as a whole and the new ones added as a whole (§10.3).
pub const COST_CAP: usize = 1 << 24;

/// How many leading bytes are searched for a NUL to tell binary content.
const BINARY_PROBE_BYTES: usize = 8192;

/// One line of a text: what stands before its line feed, and whether a line feed ended it. Only
/// the last line of a text can lack one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The line without its line feed.
    pub text: String,
    /// Whether a line feed ended the line.
    pub terminated: bool,
}

/// A line of a hunk, with what the edit does to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// In both texts; printed with a leading space.
    Context(Line),
    /// Only in the old text; printed with `-`.
    Removed(Line),
    /// Only in the new text; printed with `+`.
    Added(Line),
}

impl Row {
    const fn line(&self) -> &Line {
        match self {
            Self::Context(line) | Self::Removed(line) | Self::Added(line) => line,
        }
    }

    const fn marker(&self) -> char {
        match self {
            Self::Context(_) => ' ',
            Self::Removed(_) => '-',
            Self::Added(_) => '+',
        }
    }

    /// Whether the row is in the old text.
    #[must_use]
    pub const fn in_old(&self) -> bool {
        matches!(self, Self::Context(_) | Self::Removed(_))
    }

    /// Whether the row is in the new text.
    #[must_use]
    pub const fn in_new(&self) -> bool {
        matches!(self, Self::Context(_) | Self::Added(_))
    }
}

/// One `@@` block: where it starts and how long it is in both texts, and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// The first old line of the hunk, from 1; the line before the hunk when `old_count` is 0.
    pub old_start: usize,
    /// How many old lines the hunk covers.
    pub old_count: usize,
    /// The first new line of the hunk, from 1; the line before the hunk when `new_count` is 0.
    pub new_start: usize,
    /// How many new lines the hunk covers.
    pub new_count: usize,
    /// The context, removed and added lines in order.
    pub rows: Vec<Row>,
}

/// How two files compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// The bytes are equal.
    Unchanged,
    /// A text holds a NUL in its first 8192 bytes or is not UTF-8, and the bytes differ.
    Binary,
    /// Both are text and differ: the hunks, which are never empty.
    Text(Vec<Hunk>),
}

/// Compares two files.
#[must_use]
pub fn compare(old: &[u8], new: &[u8]) -> Comparison {
    compare_within(old, new, COST_CAP)
}

/// [`compare`] with a search that may cost `cost_cap` steps.
#[must_use]
pub fn compare_within(old: &[u8], new: &[u8], cost_cap: usize) -> Comparison {
    if old == new {
        Comparison::Unchanged
    } else {
        match (as_text(old), as_text(new)) {
            (Some(before), Some(after)) => {
                Comparison::Text(hunks(&lines(before), &lines(after), cost_cap))
            }
            _ => Comparison::Binary,
        }
    }
}

/// The text of `bytes`, unless it holds a NUL in its first 8192 bytes or is not UTF-8.
fn as_text(bytes: &[u8]) -> Option<&str> {
    let probe = bytes.get(..BINARY_PROBE_BYTES).unwrap_or(bytes);
    if probe.contains(&0) {
        None
    } else {
        std::str::from_utf8(bytes).ok()
    }
}

fn lines(text: &str) -> Vec<Line> {
    text.split_inclusive('\n')
        .map(|piece| match piece.strip_suffix('\n') {
            Some(text) => Line {
                text: text.to_owned(),
                terminated: true,
            },
            None => Line {
                text: piece.to_owned(),
                terminated: false,
            },
        })
        .collect()
}

/// A row of the edit script before it is copied: the line it shows, what the edit does to it, and
/// how many old and new lines come before it.
struct Placed<'a> {
    edit: Edit,
    line: &'a Line,
    old_before: usize,
    new_before: usize,
}

impl Placed<'_> {
    const fn in_old(&self) -> bool {
        matches!(self.edit, Edit::Keep | Edit::Remove)
    }

    const fn in_new(&self) -> bool {
        matches!(self.edit, Edit::Keep | Edit::Add)
    }

    fn row(&self) -> Row {
        let line = self.line.clone();
        match self.edit {
            Edit::Keep => Row::Context(line),
            Edit::Remove => Row::Removed(line),
            Edit::Add => Row::Added(line),
        }
    }
}

/// Every row of the edit script, each with the lines that came before it.
fn placed<'a>(old: &'a [Line], new: &'a [Line], cost_cap: usize) -> Vec<Placed<'a>> {
    let (mut old_lines, mut new_lines) = (old.iter(), new.iter());
    script(old, new, cost_cap)
        .into_iter()
        .scan((0, 0), |(old_before, new_before), edit| {
            let line = match edit {
                Edit::Keep => {
                    new_lines.next();
                    old_lines.next()
                }
                Edit::Remove => old_lines.next(),
                Edit::Add => new_lines.next(),
            };
            line.map(|line| {
                let row = Placed {
                    edit,
                    line,
                    old_before: *old_before,
                    new_before: *new_before,
                };
                *old_before += usize::from(row.in_old());
                *new_before += usize::from(row.in_new());
                row
            })
        })
        .collect()
}

fn hunks(old: &[Line], new: &[Line], cost_cap: usize) -> Vec<Hunk> {
    let rows = placed(old, new, cost_cap);
    let changed: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, placed)| placed.edit != Edit::Keep)
        .map(|(index, _)| index)
        .collect();
    // Changes whose contexts meet or overlap share a hunk.
    let groups = changed
        .iter()
        .fold(Vec::<(usize, usize)>::new(), |mut groups, &index| {
            match groups.last_mut() {
                Some((_, last)) if index - *last <= 2 * CONTEXT + 1 => *last = index,
                _ => groups.push((index, index)),
            }
            groups
        });
    groups
        .into_iter()
        .filter_map(|(first, last)| {
            let from = first.saturating_sub(CONTEXT);
            let to = (last + CONTEXT + 1).min(rows.len());
            rows.get(from..to).map(hunk)
        })
        .collect()
}

fn hunk(rows: &[Placed<'_>]) -> Hunk {
    let old_count = rows.iter().filter(|placed| placed.in_old()).count();
    let new_count = rows.iter().filter(|placed| placed.in_new()).count();
    let (old_before, new_before) = rows
        .first()
        .map_or((0, 0), |placed| (placed.old_before, placed.new_before));
    Hunk {
        old_start: old_before + usize::from(old_count > 0),
        old_count,
        new_start: new_before + usize::from(new_count > 0),
        new_count,
        rows: rows.iter().map(Placed::row).collect(),
    }
}

impl Comparison {
    /// The text `alz diff` prints for the file at `path`: the two header lines, then the hunks.
    /// Equal bytes have no hunk, which is what a mode change alone and an empty file look like.
    /// Every line is stripped of control and format characters (§10.1 class 1), so the patch is
    /// for reading, and the hunks stay the truth about the bytes.
    #[must_use]
    pub fn render(&self, path: &Stripped) -> String {
        match self {
            Self::Binary => format!("Binary file {path} changes\n"),
            Self::Unchanged => headers(path),
            Self::Text(hunks) => hunks
                .iter()
                .fold(headers(path), |text, hunk| text + &hunk.to_string()),
        }
    }
}

fn headers(path: &Stripped) -> String {
    format!("--- a/{path}\n+++ b/{path}\n")
}

impl std::fmt::Display for Hunk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            formatter,
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_count, self.new_start, self.new_count
        )?;
        self.rows.iter().try_for_each(|row| {
            let line = row.line();
            writeln!(
                formatter,
                "{}{}",
                row.marker(),
                Stripped::from_untrusted(&line.text)
            )?;
            if line.terminated {
                Ok(())
            } else {
                writeln!(formatter, "\\ No newline at end of file")
            }
        })
    }
}

#[cfg(test)]
mod tests;
