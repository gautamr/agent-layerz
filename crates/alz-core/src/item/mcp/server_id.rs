//! The id of an MCP server (spec §7.7): the key of a server in `mcp.yaml` and an entry of the
//! manifest's `mcp.enable`, one type for both.

use std::fmt;

use crate::diagnostic::{Printable, sealed::Sealed};

/// The longest server id, in characters.
const MAX_LENGTH: usize = 64;

/// Text that is not a value of an MCP field: the rule that was broken. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpRefusal {
    /// A server id outside `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$`.
    ServerId,
    /// A `transport` that is not `stdio`, `http` or `sse`.
    Transport,
}

impl McpRefusal {
    /// The rule, as the end of a sentence about a field.
    #[must_use]
    pub const fn rule(self) -> &'static str {
        match self {
            Self::ServerId => {
                "must be a server id: an ASCII letter or digit, then up to 63 ASCII letters, digits, '_' or '-'"
            }
            Self::Transport => "must be stdio, http or sse",
        }
    }
}

/// A server id: an ASCII letter or digit, then up to 63 ASCII letters, digits, `_` or `-`
/// (`^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$`). The grammar is the only thing that stands between a key
/// of an untrusted file and the merged servers, so no other key becomes a server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct McpServerId(String);

impl McpServerId {
    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for McpServerId {
    type Error = McpRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let mut characters = text.chars();
        let starts_well = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric());
        let rest_is_plain =
            characters.all(|next| next.is_ascii_alphanumeric() || next == '_' || next == '-');
        if starts_well && rest_is_plain && text.chars().count() <= MAX_LENGTH {
            Ok(Self(text.to_owned()))
        } else {
            Err(McpRefusal::ServerId)
        }
    }
}

impl fmt::Display for McpServerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Sealed for McpServerId {}

impl Printable for McpServerId {
    fn printable(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{prop_assert_eq, proptest};

    use super::{McpRefusal, McpServerId};

    #[test]
    fn ids_of_the_grammar_are_accepted() {
        for id in [
            "a",
            "Z",
            "0",
            "context7",
            "my-server_2",
            "constructor",
            "9-",
            &"a".repeat(64),
        ] {
            assert_eq!(
                McpServerId::try_from(id).map(|id| id.to_string()),
                Ok(id.to_owned())
            );
        }
    }

    /// Not a server id, whatever the key.
    #[test]
    fn anything_else_is_refused() {
        let long = "a".repeat(65);
        for key in [
            "",
            "__proto__",
            "_x",
            "-x",
            "a b",
            "a.b",
            "a/b",
            "a:b",
            "\u{e9}x",
            "a\u{e9}",
            "a\n",
            "a\0",
            "\u{ff21}",
            "prototype ",
            long.as_str(),
        ] {
            assert_eq!(
                McpServerId::try_from(key),
                Err(McpRefusal::ServerId),
                "{key:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn text_is_an_id_exactly_when_it_matches_the_grammar(text in ".{0,70}") {
            let by_hand = {
                let bytes = text.as_bytes();
                !bytes.is_empty()
                    && bytes.len() <= 64
                    && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
                    && bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
            };
            prop_assert_eq!(McpServerId::try_from(text.as_str()).is_ok(), by_hand);
        }
    }
}
