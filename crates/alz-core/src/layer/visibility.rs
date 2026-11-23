//! A layer's visibility (spec §5.1) and the privacy order the leak guard and the output
//! visibility rest on.

use thiserror::Error;

/// `public`, `private` or `confidential`. The derived order is the privacy order, from the least
/// to the most private, so the most private of several layers is their `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Visibility {
    /// May be published; the leak guard applies (§9.2).
    Public,
    /// Not published.
    Private,
    /// Not published and not shared outside the client.
    Confidential,
}

/// Text that is none of the three visibilities. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a visibility is public, private or confidential")]
pub struct VisibilityError;

impl VisibilityError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be public, private or confidential"
    }
}

impl Visibility {
    /// The word the spec writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Confidential => "confidential",
        }
    }
}

impl TryFrom<&str> for Visibility {
    type Error = VisibilityError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            "confidential" => Ok(Self::Confidential),
            _ => Err(VisibilityError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Visibility, VisibilityError};

    #[test]
    fn the_order_is_the_privacy_order() {
        assert!(Visibility::Public < Visibility::Private);
        assert!(Visibility::Private < Visibility::Confidential);
        assert_eq!(
            [Visibility::Private, Visibility::Public].into_iter().max(),
            Some(Visibility::Private)
        );
    }

    #[test]
    fn each_word_reads_back_as_itself() {
        for visibility in [
            Visibility::Public,
            Visibility::Private,
            Visibility::Confidential,
        ] {
            assert_eq!(Visibility::try_from(visibility.as_str()), Ok(visibility));
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for text in ["", "Public", "secret", "public ", "internal"] {
            assert_eq!(Visibility::try_from(text), Err(VisibilityError), "{text:?}");
        }
    }
}
