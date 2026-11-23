//! Which lock entry covers a source, and whether a layer read from the cache still matches it
//! (spec §5.3 "Cache integrity"). Pure: the layer's files arrive as a [`LayerFiles`].

use crate::lock::{ContentHash, LayerFiles, Lock, LockedLayer, ObjectId};
use crate::stack::Source;

/// The verdict of comparing a layer's files with its lock entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrity {
    /// The content hash of the files is the one the lock records.
    Matches,
    /// The files changed after they were cached.
    Differs,
}

/// The commit the lock pins `source` to: the `sha` of the entry for the same source. `None` when
/// the lock has no entry for it, or the entry has no `sha`, so there is nothing to read from the
/// cache by.
#[must_use]
pub fn locked_sha_for(lock: &Lock, source: &Source) -> Option<ObjectId> {
    lock.layers
        .iter()
        .find(|layer| &layer.source == source)
        .and_then(|layer| layer.sha)
}

/// The entry that a layer read from the cache is checked against: the one for the same `source`
/// whose `sha` equals `commit`, the commit just resolved. A moved ref (a different `sha`) or no
/// entry means new content is expected and nothing is compared.
#[must_use]
pub fn locked_entry_for<'l>(
    lock: &'l Lock,
    source: &Source,
    commit: &ObjectId,
) -> Option<&'l LockedLayer> {
    lock.layers
        .iter()
        .find(|layer| &layer.source == source && layer.sha.as_ref() == Some(commit))
}

/// Compares the content hash of `files` with the lock entry's.
#[must_use]
pub fn verify(files: &LayerFiles, locked: &LockedLayer) -> Integrity {
    if ContentHash::of(files) == locked.content_hash {
        Integrity::Matches
    } else {
        Integrity::Differs
    }
}

#[cfg(test)]
mod tests {
    use super::{Integrity, locked_entry_for, locked_sha_for, verify};
    use crate::layer::{LayerName, Visibility};
    use crate::lock::{ContentHash, LayerFiles, Lock, LockedLayer, ObjectId};
    use crate::path::SafeRelPath;
    use crate::stack::Source;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER: &str = "fedcba9876543210fedcba9876543210fedcba98";

    fn files(content: &str) -> LayerFiles {
        LayerFiles::from_contents([(
            SafeRelPath::try_from("layer.yaml").expect("a path"),
            content.as_bytes(),
        )])
    }

    fn locked(source: &str, sha: Option<&str>, files: &LayerFiles) -> LockedLayer {
        LockedLayer {
            name: LayerName::try_from("team").expect("a name"),
            source: Source::try_from(source).expect("a source"),
            rev: None,
            sha: sha.map(|text| ObjectId::try_from(text).expect("an id")),
            content_hash: ContentHash::of(files),
        }
    }

    fn lock_of(layers: Vec<LockedLayer>) -> Lock {
        Lock {
            visibility: Visibility::Public,
            layers,
            files: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn the_lock_pins_a_source_by_the_sha_of_its_own_entry() {
        let held = files("a");
        let lock = lock_of(vec![
            locked("github:o/r@v1", Some(SHA), &held),
            locked("github:o/other@v1", Some(OTHER), &held),
        ]);
        let id = |text: &str| ObjectId::try_from(text).expect("an id");
        let source = |text: &str| Source::try_from(text).expect("a source");
        assert_eq!(
            locked_sha_for(&lock, &source("github:o/r@v1")),
            Some(id(SHA))
        );
        assert_eq!(locked_sha_for(&lock, &source("github:o/r@v2")), None);
        assert!(locked_entry_for(&lock, &source("github:o/r@v1"), &id(SHA)).is_some());
        assert!(locked_entry_for(&lock, &source("github:o/r@v1"), &id(OTHER)).is_none());
        let unpinned = lock_of(vec![locked("github:o/r@v1", None, &held)]);
        assert_eq!(locked_sha_for(&unpinned, &source("github:o/r@v1")), None);
    }

    #[test]
    fn a_layer_matches_its_entry_exactly_when_the_content_hash_is_equal() {
        let held = files("a");
        let entry = locked("github:o/r@v1", Some(SHA), &held);
        assert_eq!(verify(&held, &entry), Integrity::Matches);
        assert_eq!(verify(&files("b"), &entry), Integrity::Differs);
        assert_eq!(verify(&LayerFiles::default(), &entry), Integrity::Differs);
    }
}
