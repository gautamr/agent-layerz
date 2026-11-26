//! The agent tools alz writes configuration for (spec §8). A target is named by a
//! [`TargetId`] (§5.2 **Target ids**), which is open: any well-formed id
//! is a target, and only the registered adapters render one. This module holds the two ids that
//! the code of this crate names, `claude` and `codex`, and the adapters that exist.

pub mod claude;

use std::sync::LazyLock;

use alz_adapter::TargetId;

use crate::diagnostic::{Printable, sealed::Sealed};

/// The target ids of the adapters that render today, in name order (§8 **Targets without an
/// adapter**). The Codex adapter is not among them until it exists.
pub const REGISTERED: [&str; 1] = ["claude"];

static CLAUDE: LazyLock<TargetId> = LazyLock::new(|| well_formed("claude"));
static CODEX: LazyLock<TargetId> = LazyLock::new(|| well_formed("codex"));

fn well_formed(id: &str) -> TargetId {
    TargetId::try_from(id).expect("a literal of this file is a well-formed target id")
}

impl Sealed for TargetId {}

/// A target id is printable text: its alphabet is lower-case ASCII, digits and `-`.
impl Printable for TargetId {
    fn printable(&self) -> &str {
        self.as_str()
    }
}

/// The id `claude`.
#[must_use]
pub fn claude() -> &'static TargetId {
    &CLAUDE
}

/// The id `codex`.
#[must_use]
pub fn codex() -> &'static TargetId {
    &CODEX
}

/// A target id from text that the test knows is well formed.
#[cfg(test)]
pub(crate) fn id(text: &str) -> TargetId {
    well_formed(text)
}

#[cfg(test)]
mod tests {
    use super::{REGISTERED, claude, codex};

    #[test]
    fn the_named_ids_are_their_words() {
        assert_eq!(claude().as_str(), "claude");
        assert_eq!(codex().as_str(), "codex");
        assert!(claude() < codex());
    }

    #[test]
    fn the_registered_adapters_are_listed_in_name_order() {
        let mut sorted = REGISTERED;
        sorted.sort_unstable();
        assert_eq!(REGISTERED, sorted);
    }
}
