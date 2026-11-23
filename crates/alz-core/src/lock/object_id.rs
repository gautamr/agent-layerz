//! A git object id as `layer.lock` holds it (spec §5.3): 40 hexadecimal characters for SHA-1 or 64
//! for SHA-256, lower case. A lock is untrusted, and an id builds a cache path, so any other text
//! makes the lock invalid before a path is built.

use std::fmt;

use thiserror::Error;

use crate::lock::hex;

/// A git object id: the commit a layer was resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectId {
    /// A SHA-1 id, 40 hexadecimal characters.
    Sha1([u8; 20]),
    /// A SHA-256 id, 64 hexadecimal characters.
    Sha256([u8; 32]),
}

/// Text that is not an object id. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not an object id: 40 or 64 lowercase hexadecimal characters")]
pub struct ObjectIdError;

impl ObjectIdError {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        "must be 40 or 64 lowercase hexadecimal characters"
    }
}

impl TryFrom<&str> for ObjectId {
    type Error = ObjectIdError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        hex::decode::<20>(text)
            .map(Self::Sha1)
            .or_else(|| hex::decode::<32>(text).map(Self::Sha256))
            .ok_or(ObjectIdError)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sha1(bytes) => formatter.write_str(&hex::encode(bytes)),
            Self::Sha256(bytes) => formatter.write_str(&hex::encode(bytes)),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{any, prop_assert_eq, proptest};

    use super::{ObjectId, ObjectIdError};

    const SHA1: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";

    #[test]
    fn forty_or_sixty_four_lowercase_hex_characters_are_an_id() {
        assert!(matches!(ObjectId::try_from(SHA1), Ok(ObjectId::Sha1(_))));
        assert!(matches!(
            ObjectId::try_from("ab".repeat(32).as_str()),
            Ok(ObjectId::Sha256(_))
        ));
    }

    #[test]
    fn a_sha_of_another_length_or_alphabet_is_refused() {
        let thirty_nine: String = SHA1.chars().take(39).collect();
        let forty_one = format!("{SHA1}0");
        let upper = SHA1.to_uppercase();
        let not_hex = format!("{thirty_nine}g");
        let sixty_three = "a".repeat(63);
        let sixty_five = "a".repeat(65);
        let sha_with_prefix = format!("sha1:{SHA1}");
        let padded = format!(" {SHA1}");
        let unicode_digit = format!("{thirty_nine}\u{663}");
        for text in [
            "",
            &thirty_nine,
            &forty_one,
            &upper,
            &not_hex,
            &sixty_three,
            &sixty_five,
            &sha_with_prefix,
            &padded,
            &unicode_digit,
            "../../../etc/passwd",
        ] {
            assert_eq!(ObjectId::try_from(text), Err(ObjectIdError), "{text:?}");
        }
    }

    proptest! {
        #[test]
        fn an_id_prints_back_as_it_was_read(bytes in any::<[u8; 20]>(), wide in any::<[u8; 32]>()) {
            for id in [ObjectId::Sha1(bytes), ObjectId::Sha256(wide)] {
                prop_assert_eq!(ObjectId::try_from(id.to_string().as_str()), Ok(id));
            }
        }
    }
}
