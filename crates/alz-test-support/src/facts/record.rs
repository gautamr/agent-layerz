//! The facts file of an adapter and how a row of it records a key (docs/target-facts.md).

use std::collections::BTreeSet;
use std::io;
use std::path::{Component, Path};

use alz_adapter::Descriptor;
use thiserror::Error;

/// Why the facts file of a descriptor could not be read.
#[derive(Debug, Error)]
pub enum FactsError {
    /// The descriptor's facts file is not a path below the repository.
    #[error("the facts file `{0}` is not a relative path below the repository")]
    NotBelowRepository(String),
    /// The file could not be read.
    #[error("the facts file `{file}` could not be read")]
    Read {
        /// The facts file, as the descriptor names it.
        file: String,
        /// Why it could not be read.
        #[source]
        source: io::Error,
    },
}

/// Where the facts for a kind of key are: the headings of the sections of the facts file that the
/// spec names for that kind of output, and the words a row must hold to be about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subject {
    /// The starts of the headings of the sections to look in, such as `"## 4. "`.
    pub sections: &'static [&'static str],
    /// Text a row must hold to be about this kind of output; empty for every row.
    pub row_holds: &'static str,
}

/// What an adapter's facts file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts(String);

impl Facts {
    /// Facts from their text.
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The facts file that `descriptor` names (spec §8.0 **Descriptor.**), read from `repository`,
    /// the root of the repository the path is relative to.
    ///
    /// # Errors
    ///
    /// [`FactsError::NotBelowRepository`] when the descriptor's path is absolute or leaves the
    /// repository, and [`FactsError::Read`] when the file cannot be read.
    pub fn of(descriptor: &Descriptor, repository: &Path) -> Result<Self, FactsError> {
        let relative = Path::new(&descriptor.facts_file);
        let below = !descriptor.facts_file.is_empty()
            && relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !below {
            return Err(FactsError::NotBelowRepository(
                descriptor.facts_file.clone(),
            ));
        }
        std::fs::read_to_string(repository.join(relative))
            .map(Self)
            .map_err(|source| FactsError::Read {
                file: descriptor.facts_file.clone(),
                source,
            })
    }

    /// The text of the section whose heading starts with `heading`, up to the next heading.
    #[must_use]
    pub fn section(&self, heading: &str) -> &str {
        section(&self.0, heading)
    }

    /// The rows (table rows and bullets) of the sections of `subject` that are about it and are
    /// not UNVERIFIED.
    #[must_use]
    pub fn verified_rows(&self, subject: &Subject) -> Vec<&str> {
        subject
            .sections
            .iter()
            .flat_map(|heading| self.section(heading).lines())
            .filter(|row| row.starts_with('|') || row.starts_with("- "))
            .filter(|row| row.contains(subject.row_holds))
            .filter(|row| !is_unverified(row))
            .collect()
    }

    /// Whether a verified row of `subject` names `name` in a code span.
    #[must_use]
    pub fn records_name(&self, subject: &Subject, name: &str) -> bool {
        self.verified_rows(subject)
            .iter()
            .any(|row| span_words(row).contains(name))
    }

    /// Whether any verified row of `subject` exists, which is what records a file location.
    #[must_use]
    pub fn records_location(&self, subject: &Subject) -> bool {
        !self.verified_rows(subject).is_empty()
    }
}

/// The text of the section of `facts` whose heading starts with `heading`, up to the next heading.
fn section<'a>(facts: &'a str, heading: &str) -> &'a str {
    let start = lines_with_offsets(facts)
        .find_map(|(here, line)| line.starts_with(heading).then_some(here));
    start.map_or("", |start| {
        let text = facts.get(start..).unwrap_or_default();
        let after_heading = text.find('\n').map_or(text.len(), |end| end + 1);
        let rest = text.get(after_heading..).unwrap_or_default();
        let end = lines_with_offsets(rest)
            .find_map(|(here, line)| line.starts_with('#').then_some(here))
            .unwrap_or(rest.len());
        rest.get(..end).unwrap_or_default()
    })
}

fn lines_with_offsets(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines().scan(0_usize, |offset, line| {
        let here = *offset;
        *offset += line.len() + 1;
        Some((here, line))
    })
}

/// Whether a row of the facts is marked UNVERIFIED in its status cell, the last one.
#[must_use]
pub fn is_unverified(row: &str) -> bool {
    row.trim_end()
        .trim_end_matches('|')
        .trim_end()
        .ends_with("UNVERIFIED")
}

/// The words (identifiers with `_` and `-`) of the code spans of `row`.
fn span_words(row: &str) -> BTreeSet<&str> {
    row.split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|span| {
            span.split(|character: char| {
                !(character.is_alphanumeric() || character == '_' || character == '-')
            })
        })
        .filter(|word| !word.is_empty())
        .collect()
}
