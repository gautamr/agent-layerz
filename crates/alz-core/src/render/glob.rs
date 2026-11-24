//! A path glob (spec §4.5): the patterns of `protected_paths` and the built-in ones.
//!
//! `**` spans folders, `*` and `?` do not, and a pattern with no `/` matches the file name at any
//! depth. A pattern and a path are compared after the folding of §4.3: normalisation form D, then
//! ASCII lower case, so the composed and the decomposed spelling of a name are one. The matcher is a set of positions in the pattern advanced
//! once per character of the path, so it runs in time linear in the pattern times the path
//! whatever the pattern is, and it is total: every pattern is a glob.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::canonical_combining_class;

use crate::stack::ProtectedPattern;

/// One step of a pattern. `**/` is two steps, so that "nothing" is allowed only before the first
/// character it would match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    /// This character, in normalisation form D and as ASCII lower case.
    Literal(char),
    /// `?`: one character other than `/`.
    AnyOne,
    /// `*`: any run of characters other than `/`.
    Star,
    /// The end of `?`: any run of combining marks, so that `?` matches `é` whether it is one
    /// scalar value or `e` with U+0301 after the decomposition.
    Marks,
    /// `**` not followed by `/`: any run of characters.
    Deep,
    /// The start of `**/`: nothing, or the first character of a run that ends in `/`.
    FoldersStart,
    /// The rest of `**/`: any characters up to and including a `/`.
    FoldersBody,
}

impl Token {
    /// The position reached without reading a character, from this token at `at`.
    fn skip(self, at: usize) -> Option<usize> {
        match self {
            Self::Star | Self::Deep | Self::Marks => Some(at.saturating_add(1)),
            Self::FoldersStart => Some(at.saturating_add(2)),
            Self::Literal(_) | Self::AnyOne | Self::FoldersBody => None,
        }
    }

    /// The positions reached when this token at `at` reads `character` (already folded).
    fn after(self, at: usize, character: char) -> [Option<usize>; 2] {
        let next = at.saturating_add(1);
        let not_slash = character != '/';
        match self {
            Self::Literal(expected) => [(character == expected).then_some(next), None],
            Self::AnyOne => [not_slash.then_some(next), None],
            Self::Star => [not_slash.then_some(at), None],
            Self::Marks => [is_combining_mark(character).then_some(at), None],
            Self::Deep => [Some(at), None],
            Self::FoldersStart => [
                Some(next),
                (character == '/').then_some(at.saturating_add(2)),
            ],
            Self::FoldersBody => [Some(at), (character == '/').then_some(next)],
        }
    }
}

/// Whether `character` is a combining mark: it has a canonical combining class.
fn is_combining_mark(character: char) -> bool {
    canonical_combining_class(character) != 0
}

/// What a pattern is matched against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subject {
    /// The whole path: the pattern has a `/`.
    WholePath,
    /// The last segment: the pattern has none.
    FileName,
}

/// A parsed glob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGlob {
    subject: Subject,
    tokens: Vec<Token>,
}

impl PathGlob {
    /// Reads `pattern`, in normalisation form D. A leading `/` is dropped, but it still makes the
    /// pattern match the whole path. A `**` before a `/` takes it, so `a/**/c` matches `a/c`.
    #[must_use]
    pub fn parse(pattern: &str) -> Self {
        let subject = if pattern.contains('/') {
            Subject::WholePath
        } else {
            Subject::FileName
        };
        let characters: Vec<char> = pattern.trim_start_matches('/').nfd().collect();
        Self {
            subject,
            tokens: tokens_of(&characters),
        }
    }

    /// Whether the glob matches `path`, a repository-relative path with `/` separators.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        let subject = match self.subject {
            Subject::WholePath => path,
            Subject::FileName => path.rsplit('/').next().unwrap_or(path),
        };
        let start = self.close(
            std::iter::once(true)
                .chain(self.tokens.iter().map(|_| false))
                .collect(),
        );
        subject
            .nfd()
            .map(|character| character.to_ascii_lowercase())
            .fold(start, |active, character| self.step(&active, character))
            .last()
            .copied()
            .unwrap_or(false)
    }

    /// Adds, to the positions in `active`, those reached without reading a character.
    fn close(&self, active: Vec<bool>) -> Vec<bool> {
        self.tokens
            .iter()
            .enumerate()
            .fold(active, |mut active, (at, token)| {
                let target = token
                    .skip(at)
                    .filter(|_| active.get(at).copied().unwrap_or(false));
                if let Some(slot) = target.and_then(|target| active.get_mut(target)) {
                    *slot = true;
                }
                active
            })
    }

    /// The positions reached from `active` by reading `character`.
    fn step(&self, active: &[bool], character: char) -> Vec<bool> {
        let reached = self
            .tokens
            .iter()
            .enumerate()
            .filter(|(at, _)| active.get(*at).copied().unwrap_or(false))
            .flat_map(|(at, token)| token.after(at, character))
            .flatten()
            .fold(vec![false; active.len()], |mut reached, position| {
                if let Some(slot) = reached.get_mut(position) {
                    *slot = true;
                }
                reached
            });
        self.close(reached)
    }
}

impl From<&ProtectedPattern> for PathGlob {
    fn from(pattern: &ProtectedPattern) -> Self {
        Self::parse(pattern.as_str())
    }
}

/// The tokens of a pattern, read from its start. Iterative, because a pattern is input.
fn tokens_of(characters: &[char]) -> Vec<Token> {
    std::iter::successors(split_token(characters), |(_, rest)| split_token(rest))
        .flat_map(|(tokens, _)| tokens)
        .flatten()
        .collect()
}

/// The tokens of the first element of `characters`, and what follows it.
fn split_token(characters: &[char]) -> Option<([Option<Token>; 2], &[char])> {
    match characters {
        [] => None,
        ['*', '*', '/', rest @ ..] => {
            Some(([Some(Token::FoldersStart), Some(Token::FoldersBody)], rest))
        }
        ['*', '*', rest @ ..] => Some(([Some(Token::Deep), None], rest)),
        ['*', rest @ ..] => Some(([Some(Token::Star), None], rest)),
        ['?', rest @ ..] => Some(([Some(Token::AnyOne), Some(Token::Marks)], rest)),
        [other, rest @ ..] => Some((
            [Some(Token::Literal(other.to_ascii_lowercase())), None],
            rest,
        )),
    }
}

#[cfg(test)]
mod reference_table;
#[cfg(test)]
mod tests;
