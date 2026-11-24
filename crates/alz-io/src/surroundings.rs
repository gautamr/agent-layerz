//! What is on disk around a repository, for `alz doctor` (spec §10.3): the client root of a hub,
//! the workspace directories and the folders above the repository. Each question is whether
//! something exists (`stat`, never an open) or what a folder holds by name.

use std::path::{Component, Path, PathBuf};

use alz_core::Stripped;
use alz_core::doctor::{Ancestor, ClientRoot, WorkspaceFolder, WorkspaceState};

fn shown(path: &Path) -> Stripped {
    Stripped::from_untrusted(&path.to_string_lossy())
}

/// `base` joined with `relative`, with `.` dropped and each `..` applied lexically, so a path
/// that goes through a link is judged as it is written (as the manifest means it).
#[must_use]
pub fn resolve_below(base: &Path, relative: &Path) -> PathBuf {
    base.join(relative)
        .components()
        .fold(PathBuf::new(), |mut resolved, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                    resolved.push(component);
                }
            }
            resolved
        })
}

/// The folder above the folder that `hub` names, which is where a client keeps its repositories.
#[must_use]
pub fn client_root_of(root: &Path, hub: &Path) -> Option<PathBuf> {
    resolve_below(root, hub).parent().map(Path::to_path_buf)
}

fn exists(path: &Path) -> bool {
    path.exists()
}

/// What the client root holds.
#[must_use]
pub fn observe_client_root(folder: &Path) -> ClientRoot {
    ClientRoot {
        path: shown(folder),
        has_claude_md: exists(&folder.join("CLAUDE.md")),
        has_agents_md: exists(&folder.join("AGENTS.md")),
        is_git_repository: exists(&folder.join(".git")),
    }
}

/// A workspace directory `declared` in the manifest, below `root`.
#[must_use]
pub fn observe_workspace(root: &Path, declared: &str) -> WorkspaceFolder {
    let resolved = resolve_below(root, Path::new(declared));
    let state = if exists(&resolved) {
        WorkspaceState::Present {
            loaded: ["skills", "agents", "commands"]
                .into_iter()
                .filter(|kind| exists(&resolved.join(".claude").join(kind)))
                .collect(),
        }
    } else {
        WorkspaceState::Missing
    };
    WorkspaceFolder {
        declared: Stripped::from_untrusted(declared),
        resolved: shown(&resolved),
        state,
    }
}

/// The folders above `root`, nearest first, without the file system root and without
/// `client_root`.
#[must_use]
pub fn observe_ancestors(root: &Path, client_root: Option<&Path>) -> Vec<Ancestor> {
    root.ancestors()
        .skip(1)
        .filter(|folder| folder.parent().is_some())
        .filter(|folder| Some(*folder) != client_root)
        .map(|folder| Ancestor {
            path: shown(folder),
            has_claude_md: exists(&folder.join("CLAUDE.md")),
            is_git_repository_with_claude: exists(&folder.join(".git"))
                && exists(&folder.join(".claude")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{client_root_of, resolve_below};

    #[test]
    fn dots_are_applied_lexically() {
        assert_eq!(
            resolve_below(Path::new("/w/c/repo"), Path::new("../hub/./layer")),
            PathBuf::from("/w/c/hub/layer")
        );
        assert_eq!(
            resolve_below(Path::new("/w"), Path::new("../../..")),
            PathBuf::from("/")
        );
    }

    #[test]
    fn the_client_root_is_above_the_hub_folder() {
        assert_eq!(
            client_root_of(Path::new("/w/c/repo"), Path::new("../hub")),
            Some(PathBuf::from("/w/c"))
        );
    }
}
