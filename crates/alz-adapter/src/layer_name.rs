//! The name of a layer (spec §5.1): lower-case ASCII letters and digits in groups joined by a
//! single `-` or `.`, that is `^[a-z0-9]+(?:[-.][a-z0-9]+)*$`. The model names the layers that set
//! a security item, and the sources of a header, with it.

use std::fmt;

use thiserror::Error;

/// A layer name. Its alphabet is printable ASCII, so it goes into a message as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerName(String);

/// Text that is not a layer name. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a layer name: lower-case letters and digits in groups joined by a single '-' or '.'")]
pub struct LayerNameError;

impl LayerName {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn is_group_character(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

impl TryFrom<&str> for LayerName {
    type Error = LayerNameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let well_formed = text
            .split(['-', '.'])
            .all(|group| !group.is_empty() && group.bytes().all(is_group_character));
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(LayerNameError)
        }
    }
}

impl fmt::Display for LayerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::LayerName;

    #[test]
    fn accepts_names_in_groups_joined_by_one_separator() {
        for text in ["personal", "a", "client-a", "org.team", "a1-b2.c3", "0"] {
            assert!(LayerName::try_from(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn refuses_what_the_pattern_refuses() {
        for text in [
            "", "-", "-a", "a-", "a--b", "a..b", "A", "a_b", "a/b", "é", "a\n",
        ] {
            assert!(LayerName::try_from(text).is_err(), "{text:?}");
        }
    }
}
