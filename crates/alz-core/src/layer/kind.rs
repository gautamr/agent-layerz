//! A layer's kind (spec §5.1, §7.12): a free-form label, of which only `client` has a meaning.

use thiserror::Error;

/// What a layer calls itself. `client` is the one label alz acts on (§7.12); every other label is
/// kept as it was written and means nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// `client`: the layer may override the commit identity and the account (§7.12, §7.13).
    Client,
    /// Any other label.
    Other(KindLabel),
}

/// A non-empty kind label other than `client`. Only [`Kind`] makes one, so `client` is never an
/// `Other`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KindLabel(String);

impl KindLabel {
    /// The label as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A kind that is not text with at least one character. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a layer kind must not be empty")]
pub struct KindError;

impl KindError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must not be empty"
    }
}

impl Kind {
    /// Whether the kind is `client`.
    #[must_use]
    pub const fn is_client(&self) -> bool {
        matches!(self, Self::Client)
    }

    /// The label as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Client => "client",
            Self::Other(label) => label.as_str(),
        }
    }
}

impl Kind {
    /// The label with its first character in capitals, as the heading of a layer's text writes it:
    /// `repo` is `Repo` (§7.1).
    #[must_use]
    pub fn title(&self) -> String {
        let label = self.as_str();
        let mut characters = label.chars();
        characters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(characters).collect()
        })
    }
}

impl TryFrom<&str> for Kind {
    type Error = KindError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "" => Err(KindError),
            "client" => Ok(Self::Client),
            other => Ok(Self::Other(KindLabel(other.to_owned()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, KindError};

    #[test]
    fn only_client_has_a_meaning() {
        assert_eq!(Kind::try_from("client"), Ok(Kind::Client));
        assert!(Kind::try_from("personal").is_ok_and(|kind| !kind.is_client()));
        assert!(Kind::try_from("Client").is_ok_and(|kind| !kind.is_client()));
    }

    #[test]
    fn the_title_capitalises_the_first_character_only() {
        let title = |label: &str| Kind::try_from(label).expect("a kind").title();
        assert_eq!(title("repo"), "Repo");
        assert_eq!(title("client"), "Client");
        assert_eq!(title("my Kind"), "My Kind");
        assert_eq!(title("Team"), "Team");
    }

    #[test]
    fn a_label_is_kept_as_written() {
        let kind = Kind::try_from("my kind, free-form").expect("any text is a label");
        assert_eq!(kind.as_str(), "my kind, free-form");
    }

    #[test]
    fn an_empty_kind_is_refused() {
        assert_eq!(Kind::try_from(""), Err(KindError));
    }
}
