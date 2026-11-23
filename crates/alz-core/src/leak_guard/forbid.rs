//! A layer's `forbid_patterns` (spec §9.2): each entry compiled with `regex-lite` under the limits
//! of decision-38, or refused with a message that names the entry by its number and never holds
//! its text.

use std::num::NonZeroUsize;

use regex_lite::Regex;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::ForbidPatternText;
use crate::secret::bounded;

/// The most entries a layer may have.
pub(super) const MAX_ENTRIES: usize = 64;

/// The most characters an entry may have.
pub(super) const MAX_ENTRY_CHARS: usize = 512;

/// The number of an entry as a person counts: `#1` is the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EntryNumber(NonZeroUsize);

impl EntryNumber {
    /// The number of the entry at `index`, counted from 0.
    fn of_index(index: usize) -> Option<Self> {
        index.checked_add(1).and_then(NonZeroUsize::new).map(Self)
    }

    pub(super) const fn get(self) -> usize {
        self.0.get()
    }
}

/// Why an entry is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// A look-around, which `regex-lite` does not support.
    LookAround,
    /// A backreference, which `regex-lite` does not support.
    Backreference,
    /// More than [`MAX_ENTRY_CHARS`] characters.
    TooLong,
    /// Not valid, or past the limit on the compiled size or on the nesting.
    NotCompilable,
    /// The first entry past [`MAX_ENTRIES`].
    TooMany,
}

impl Refusal {
    fn message(self, number: EntryNumber) -> Message {
        let entry = Message::new("forbid_patterns entry #").then(number.get());
        match self {
            Self::LookAround => {
                entry.then(" uses look-around, which is not supported by the pattern syntax")
            }
            Self::Backreference => {
                entry.then(" uses a backreference, which is not supported by the pattern syntax")
            }
            Self::TooLong => entry.then(" is longer than 512 characters"),
            Self::NotCompilable => entry.then(
                " is not a valid pattern, or compiles to more than 64 KiB or nests more than 32 levels",
            ),
            Self::TooMany => entry
                .then(" is past the 64 entries a layer may have; it and every later entry are refused"),
        }
    }
}

/// The entries of one layer: those that compile, and those that are refused.
#[derive(Debug)]
pub(super) struct ForbidPatterns {
    compiled: Vec<(EntryNumber, Regex)>,
    refused: Vec<(EntryNumber, Refusal)>,
}

impl ForbidPatterns {
    /// Compiles the entries in order. Only the first [`MAX_ENTRIES`] are compiled; the next one is
    /// refused and the rest are left alone, so the work is bounded.
    pub(super) fn compile(entries: &[ForbidPatternText]) -> Self {
        let (compiled, refused) = entries
            .iter()
            .take(MAX_ENTRIES.saturating_add(1))
            .enumerate()
            .filter_map(|(index, entry)| EntryNumber::of_index(index).map(|number| (number, entry)))
            .map(|(number, entry)| {
                if number.get() > MAX_ENTRIES {
                    Err((number, Refusal::TooMany))
                } else {
                    compile_entry(entry.as_str())
                        .map(|regex| (number, regex))
                        .map_err(|refusal| (number, refusal))
                }
            })
            .fold((Vec::new(), Vec::new()), |(mut ok, mut bad), result| {
                match result {
                    Ok(pattern) => ok.push(pattern),
                    Err(refusal) => bad.push(refusal),
                }
                (ok, bad)
            });
        Self { compiled, refused }
    }

    /// One error `leak-forbid` for each refused entry, at `at`, in entry order.
    pub(super) fn refusals(&self, at: &Where) -> Diagnostics {
        self.refused
            .iter()
            .map(|(number, refusal)| {
                Diagnostic::error(Code::LeakForbid, Some(at.clone()), refusal.message(*number))
            })
            .collect()
    }

    /// The numbers of the entries that match `text`, in entry order.
    pub(super) fn matching<'a>(&'a self, text: &'a str) -> impl Iterator<Item = EntryNumber> + 'a {
        self.compiled
            .iter()
            .filter(move |(_, regex)| regex.is_match(text))
            .map(|(number, _)| *number)
    }
}

fn compile_entry(entry: &str) -> Result<Regex, Refusal> {
    if entry.chars().nth(MAX_ENTRY_CHARS).is_some() {
        Err(Refusal::TooLong)
    } else if let Some(refusal) = unsupported_feature(entry) {
        Err(refusal)
    } else {
        bounded(entry).map_err(|_not_compilable| Refusal::NotCompilable)
    }
}

/// The look-around or backreference that `pattern` uses, if any. An escaped character and a
/// character class are skipped, so `\\1` and `[(?=]` are plain text. This is a scan for the
/// message only: whatever it lets through, `regex-lite` still refuses.
fn unsupported_feature(pattern: &str) -> Option<Refusal> {
    let mut chars = pattern.chars().peekable();
    let mut in_class = false;
    while let Some(current) = chars.next() {
        match (current, in_class) {
            ('\\', _) => match chars.next() {
                Some('1'..='9') if !in_class => return Some(Refusal::Backreference),
                Some('k') if !in_class && chars.peek() == Some(&'<') => {
                    return Some(Refusal::Backreference);
                }
                Some(_) | None => {}
            },
            ('[', false) => {
                in_class = true;
                // `[^]...]` and `[]...]` hold a literal `]` first.
                chars.next_if_eq(&'^');
                chars.next_if_eq(&']');
            }
            (']', true) => in_class = false,
            ('(', false) if starts_look_around(chars.clone()) => return Some(Refusal::LookAround),
            _ => {}
        }
    }
    None
}

/// Whether the text after a `(` is `?=`, `?!`, `?<=` or `?<!`.
fn starts_look_around(after_paren: impl Iterator<Item = char>) -> bool {
    let head: String = after_paren.take(3).collect();
    ["?=", "?!", "?<=", "?<!"]
        .iter()
        .any(|opening| head.starts_with(opening))
}
