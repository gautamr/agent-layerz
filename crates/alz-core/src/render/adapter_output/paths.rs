//! Where an adapter may write (spec §8.0 **Output checks.**): inside its territory, and never at a
//! path that only the core writes.

use alz_adapter::Territory;

use crate::diagnostic::Message;
use crate::render::tree::{OutputPath, fold_text};
use crate::untrusted_text::Stripped;

/// The files that only the core writes, as folded paths of the repository root (§4.3, §4.5).
const CORE_FILES: [&str; 3] = ["agents.md", "layer.yaml", "layer.lock"];

/// The folders below which only the core writes, folded, each with its closing `/`.
const CORE_FOLDERS: [&str; 2] = [".layer/", ".git/"];

/// Why a path is not one the adapter may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// Neither a file of the territory nor below one of its folders.
    OutsideTerritory,
    /// `AGENTS.md`, `layer.yaml`, `layer.lock`, or a path below `.layer/` or `.git/`.
    CoreOnly,
}

impl Refusal {
    /// The rule that `path` broke, for the message of the error `target`.
    pub(super) fn rule(self, path: &OutputPath) -> Message {
        let shown = Stripped::from_untrusted(path.as_str());
        match self {
            Self::OutsideTerritory => Message::new("the path ")
                .then(shown)
                .then(" is outside the adapter's territory"),
            Self::CoreOnly => Message::new("the path ")
                .then(shown)
                .then(" is written by the core only"),
        }
    }
}

/// `path` when the adapter may write it. Paths are compared after the folding of §4.3, as the
/// protected paths are (§4.5), so `agents.md` is `AGENTS.md`. A path that only the core writes is
/// refused first, whatever the territory says.
pub(super) fn check(territory: &Territory, path: &OutputPath) -> Result<(), Refusal> {
    let folded = path.folded();
    if is_core_only(&folded) {
        Err(Refusal::CoreOnly)
    } else if in_territory(territory, &folded) {
        Ok(())
    } else {
        Err(Refusal::OutsideTerritory)
    }
}

fn is_core_only(folded: &str) -> bool {
    CORE_FILES.contains(&folded) || CORE_FOLDERS.iter().any(|folder| folded.starts_with(folder))
}

/// Whether the folded path is a file of the territory or lies below one of its folders.
fn in_territory(territory: &Territory, folded: &str) -> bool {
    territory.files.iter().any(|file| fold_text(file) == folded)
        || territory.folders.iter().any(|folder| {
            let root = fold_text(folder);
            folded
                .strip_prefix(root.trim_end_matches('/'))
                .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1)
        })
}
