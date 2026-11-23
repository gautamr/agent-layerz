//! The lock of a build (spec §5.3): the hash of every generated file, and each layer as it was
//! resolved.

use crate::layer::Visibility;
use crate::lock::{ContentHash, FileHash, Lock, LockedLayer};
use crate::render::tree::FileTree;
use crate::stack::ResolvedStack;

/// The lock for `tree` built from `stack`: the most private visibility of the stack, each layer
/// with its source, pin and content hash in stack order, and the hash of each file's bytes.
///
/// Write it with [`Lock::emit`].
#[must_use]
pub fn lock_of(tree: &FileTree, stack: &ResolvedStack) -> Lock {
    Lock {
        visibility: stack
            .layers()
            .iter()
            .map(|layer| layer.meta().visibility())
            .max()
            .unwrap_or(Visibility::Public),
        layers: stack
            .layers()
            .iter()
            .map(|layer| LockedLayer {
                name: layer.meta().name().clone(),
                source: layer.source().clone(),
                rev: layer.pin().map(|pin| pin.rev.clone()),
                sha: layer.pin().map(|pin| pin.sha),
                content_hash: ContentHash::of(layer.files()),
            })
            .collect(),
        files: tree
            .iter()
            .map(|(path, file)| (path.as_safe().clone(), FileHash::of(&file.bytes)))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
