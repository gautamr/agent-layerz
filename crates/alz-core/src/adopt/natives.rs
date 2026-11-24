//! Which native files adoption takes (spec §4.3): the build's collisions that alz did not
//! generate, and the files that are always adopted when alz did not generate them. Pure: the
//! caller observed the files.

use std::collections::BTreeMap;

use crate::lock::Lock;
use crate::path::SafeRelPath;
use crate::render::write_plan::{Collision, CollisionReason, Disk, Observed, owns};

use super::NativeFile;

/// The files that are adopted even when the build does not collide with them.
pub const ADOPT_ALWAYS: [&str; 2] = [".claude/settings.json", ".mcp.json"];

/// The file whose paragraphs `CLAUDE.md` shares.
const AGENTS_MD: &str = "AGENTS.md";

/// What adoption takes, and what stops it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidates {
    natives: BTreeMap<SafeRelPath, NativeFile>,
    agents_md: Option<Vec<u8>>,
    unadoptable: Vec<Collision>,
}

impl Candidates {
    /// The native files to adopt, with the bytes observed.
    #[must_use]
    pub const fn natives(&self) -> &BTreeMap<SafeRelPath, NativeFile> {
        &self.natives
    }

    /// The bytes of `AGENTS.md` as it is on disk, when it is a regular file.
    #[must_use]
    pub fn agents_md(&self) -> Option<&[u8]> {
        self.agents_md.as_deref()
    }

    /// The collisions adoption cannot clear: a link, a file that is not a regular file, a file too
    /// large to compare, an input of alz. A build with one of them fails whatever is adopted, so
    /// nothing is adopted.
    #[must_use]
    pub fn unadoptable(&self) -> &[Collision] {
        &self.unadoptable
    }
}

/// Picks the files adoption takes. `beside` holds what was observed at [`ADOPT_ALWAYS`] and at
/// `AGENTS.md`, which the build may not have looked at; an entry that is not a regular file there
/// is left alone, because a plain build would leave it alone too.
#[must_use]
pub fn candidates(
    collisions: &[Collision],
    disk: &Disk,
    beside: &BTreeMap<SafeRelPath, Observed>,
    previous: Option<&Lock>,
) -> Candidates {
    let (collided, unadoptable): (Vec<_>, Vec<_>) = collisions.iter().partition(|collision| {
        matches!(collision.reason(), CollisionReason::NotGenerated)
            && matches!(disk.get(collision.path()), Observed::Regular { .. })
    });
    let from_collisions = collided
        .into_iter()
        .filter_map(|collision| native_of(collision.path(), disk.get(collision.path())));
    let always = beside
        .iter()
        .filter(|(path, _)| ADOPT_ALWAYS.contains(&path.as_str()))
        .filter(|(path, observed)| {
            !matches!(observed, Observed::Regular { bytes, .. }
                if owns(bytes, previous.and_then(|lock| lock.files.get(*path))))
        })
        .filter_map(|(path, observed)| native_of(path, observed));
    Candidates {
        natives: from_collisions.chain(always).collect(),
        agents_md: beside
            .iter()
            .find(|(path, _)| path.as_str() == AGENTS_MD)
            .and_then(|(_, observed)| match observed {
                Observed::Regular { bytes, .. } => Some(bytes.clone()),
                Observed::Absent | Observed::Link | Observed::NotRegular | Observed::TooLarge => {
                    None
                }
            }),
        unadoptable: unadoptable.into_iter().cloned().collect(),
    }
}

fn native_of(path: &SafeRelPath, observed: &Observed) -> Option<(SafeRelPath, NativeFile)> {
    match observed {
        Observed::Regular { bytes, executable } => {
            Some((path.clone(), NativeFile::new(bytes.clone(), *executable)))
        }
        Observed::Absent | Observed::Link | Observed::NotRegular | Observed::TooLarge => None,
    }
}
