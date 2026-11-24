//! The repository root of the working directory (spec §7.13 "Paths"; decision-19).
//!
//! The root is the nearest directory at or above the working directory whose `layer.yaml` has a
//! `stack` key. A `layer.yaml` without one, such as a layer's own manifest, is skipped. One that
//! cannot be used stops the walk: that directory is the root, with one warning that names the
//! file, and the walk never goes on past it, so a nested checkout with a broken manifest never
//! gets the outer repository's account. The walk never opens a `layer.yaml` at `$HOME`, above it
//! or in an account folder, and it opens each one through no link and without waiting for a
//! writer.
//!
//! The walk takes the [`Place`] by value. It climbs by the parents of a real path, which are real
//! paths too, so it resolves nothing again.

use alz_core::account::{AccountFolders, Relation};
use alz_core::lint::{is_stack_manifest, parse_document};
use alz_core::path::AbsolutePath;
use alz_core::stack::Manifest;
use alz_core::yaml::MAX_BYTES;
use alz_core::{Code, Diagnostic, Diagnostics, Message};

use crate::account::files::display;
use crate::account::place::Place;
use crate::fs::{ReadError, ReadOnlyDir};

/// Why there is no repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoRoot {
    /// No directory from the working directory up to `$HOME` has a stack manifest, or the walk
    /// reached `$HOME` or above first.
    NotFound,
    /// The working directory is inside an account folder.
    AccountFolder,
    /// The working directory is inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names: no
    /// manifest is read there. The real path of that folder.
    ConfiguredFolder(AbsolutePath),
    /// The working directory does not resolve.
    Unresolvable,
}

/// What the manifest of a root is.
#[derive(Debug, Clone, PartialEq)]
pub enum ManifestState {
    /// A stack manifest that passes the schema.
    Read(Box<Manifest>),
    /// The file is a link, a folder, over 1 MiB, cannot be read, is not valid YAML or is past the
    /// alias limit.
    Unreadable,
    /// The file is a stack manifest that does not pass the schema.
    Invalid,
}

/// A repository root and what its `layer.yaml` is.
#[derive(Debug, Clone, PartialEq)]
pub struct Root {
    path: AbsolutePath,
    manifest: ManifestState,
}

impl Root {
    /// The real path of the root.
    #[must_use]
    pub const fn path(&self) -> &AbsolutePath {
        &self.path
    }

    /// What its `layer.yaml` is.
    #[must_use]
    pub const fn manifest(&self) -> &ManifestState {
        &self.manifest
    }
}

/// The repository of the working directory, or why there is none.
#[derive(Debug, Clone, PartialEq)]
pub enum Repository {
    /// A root.
    Root(Root),
    /// No repository.
    NoRoot(NoRoot),
}

/// The result of the root walk: the repository, the deepest real directory it knows, and the
/// warnings.
#[derive(Debug, Clone)]
pub struct RootSearch {
    known_directory: Option<AbsolutePath>,
    repository: Repository,
    warnings: Diagnostics,
}

impl RootSearch {
    /// The repository, or why there is none.
    #[must_use]
    pub const fn repository(&self) -> &Repository {
        &self.repository
    }

    /// The real working directory, or for a working directory inside a configured folder that
    /// folder, which is as far as anything was looked at. `None` when the place does not resolve,
    /// is inside an account folder, or is `$HOME` or above it.
    #[must_use]
    pub const fn known_directory(&self) -> Option<&AbsolutePath> {
        self.known_directory.as_ref()
    }

    /// The warnings of the walk: one for a `layer.yaml` that cannot be used.
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
    }
}

/// Finds the root of `place`, reading each `layer.yaml` through `files`, a handle on `/`. `homes`
/// are the spellings of `$HOME` that a warning shows as `~`.
#[must_use]
pub fn find_root(
    place: Place,
    folders: &AccountFolders,
    homes: &[&AbsolutePath],
    files: &ReadOnlyDir,
) -> RootSearch {
    let none = |known_directory: Option<AbsolutePath>, why: NoRoot| RootSearch {
        known_directory,
        repository: Repository::NoRoot(why),
        warnings: Diagnostics::new(),
    };
    match place {
        Place::InAccountFolder => none(None, NoRoot::AccountFolder),
        Place::InConfiguredFolder(folder) => {
            none(Some(folder.clone()), NoRoot::ConfiguredFolder(folder))
        }
        Place::AtOrAboveHome => none(None, NoRoot::NotFound),
        Place::Unresolvable => none(None, NoRoot::Unresolvable),
        Place::Directory(real) => {
            let verdict =
                ancestors(&real).find_map(|directory| decide(&directory, folders, homes, files));
            match verdict {
                Some(Verdict::Root(root, warning)) => RootSearch {
                    known_directory: Some(real),
                    repository: Repository::Root(root),
                    warnings: warning
                        .into_iter()
                        .fold(Diagnostics::new(), Diagnostics::with),
                },
                Some(Verdict::Stop) | None => none(Some(real), NoRoot::NotFound),
            }
        }
    }
}

/// `directory` and each parent of it, the deepest first, ending with `/`.
fn ancestors(directory: &AbsolutePath) -> impl Iterator<Item = AbsolutePath> + '_ {
    let names: Vec<&str> = directory.components().collect();
    (0..=names.len()).rev().filter_map(move |count| {
        let joined: Vec<&str> = names.iter().copied().take(count).collect();
        AbsolutePath::try_from(format!("/{}", joined.join("/")).as_str()).ok()
    })
}

enum Verdict {
    /// The walk ends here: no repository.
    Stop,
    /// The walk ends here with a root, and perhaps a warning.
    Root(Root, Option<Diagnostic>),
}

/// What one directory on the way decides, or `None` to go on to its parent.
fn decide(
    directory: &AbsolutePath,
    folders: &AccountFolders,
    homes: &[&AbsolutePath],
    files: &ReadOnlyDir,
) -> Option<Verdict> {
    // `$HOME` and above it are never opened; a folder that only holds a configured folder is an
    // ordinary candidate.
    if folders
        .relation(directory)
        .is_some_and(|relation| relation != Relation::Holds)
    {
        return Some(Verdict::Stop);
    }
    let state = match layer_yaml_of(directory).map(|file| files.read_absolute(&file, MAX_BYTES)) {
        None => ManifestState::Unreadable,
        Some(Err(ReadError::Missing)) => return None,
        Some(Err(ReadError::NotRegular | ReadError::TooLarge | ReadError::Io(_))) => {
            ManifestState::Unreadable
        }
        Some(Ok(file)) => match parse_document(file.bytes()) {
            Err(_unusable) => ManifestState::Unreadable,
            Ok(document) if !is_stack_manifest(&document) => return None,
            Ok(document) => Manifest::from_yaml(&document)
                .map_or(ManifestState::Invalid, |manifest| {
                    ManifestState::Read(Box::new(manifest))
                }),
        },
    };
    let warning = warning_of(&state, directory, homes);
    Some(Verdict::Root(
        Root {
            path: directory.clone(),
            manifest: state,
        },
        warning,
    ))
}

fn layer_yaml_of(directory: &AbsolutePath) -> Option<AbsolutePath> {
    let base = directory.as_str().trim_end_matches('/');
    AbsolutePath::try_from(format!("{base}/layer.yaml").as_str()).ok()
}

/// The one warning of a root whose manifest cannot be used, naming the file and why, and holding
/// no text of it.
fn warning_of(
    state: &ManifestState,
    directory: &AbsolutePath,
    homes: &[&AbsolutePath],
) -> Option<Diagnostic> {
    let why = match state {
        ManifestState::Read(_) => return None,
        ManifestState::Unreadable => "it cannot be read, parsed or is too large",
        ManifestState::Invalid => "its stack is not valid",
    };
    let file = layer_yaml_of(directory)
        .map_or_else(|| display(directory, homes), |file| display(&file, homes));
    Some(Diagnostic::warning(
        Code::AccountManifest,
        None,
        Message::new(file)
            .then(" cannot be used (")
            .then(why)
            .then("); the walk for the repository stops here"),
    ))
}
