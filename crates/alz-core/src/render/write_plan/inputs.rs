//! The folders of alz's own inputs that no build writes or deletes (spec §4.3).

use crate::path::SafeRelPath;
use crate::render::tree::fold;

/// The manifest at the root and everything under `.layer/` are always inputs; this adds the folder
/// of every repository layer whose `path:` source lies inside the repository, which the stack
/// names. Paths are compared after the case folding of §4.3.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inputs {
    /// The folders, folded.
    folders: Vec<String>,
}

impl Inputs {
    /// The inputs of a build whose stack has repository layers in `folders`.
    #[must_use]
    pub fn new(folders: impl IntoIterator<Item = SafeRelPath>) -> Self {
        Self {
            folders: folders.into_iter().map(|folder| fold(&folder)).collect(),
        }
    }

    /// Whether `path` is the manifest, lies under `.layer/` or under a repository layer's folder,
    /// or is such a folder itself.
    #[must_use]
    pub(crate) fn covers(&self, path: &SafeRelPath) -> bool {
        let key = fold(path);
        key == "layer.yaml"
            || key.starts_with(".layer/")
            || self.folders.iter().any(|folder| {
                key == *folder
                    || key
                        .strip_prefix(folder.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            })
    }
}
