//! The key of a `folders` entry in `accounts.yaml` (spec §7.13 "User config"): an absolute path
//! or a path that starts with `~/`. Where `~` leads and what the path resolves to are the
//! capability layer's business; the type holds what the text shows.

use thiserror::Error;

use crate::path::AbsolutePath;

/// The part of a `~/...` key after `~/`. It may be empty (`~/` names `$HOME` itself, which the
/// resolver then ignores).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HomeRelative(String);

impl HomeRelative {
    /// The text after `~/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A folder as `accounts.yaml` writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FolderKey {
    /// An absolute path.
    Absolute(AbsolutePath),
    /// A path that starts with `~/`, below the user's home.
    Home(HomeRelative),
}

/// Text that is neither an absolute path nor one that starts with `~/`. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a folder: an absolute path or one that starts with '~/'")]
pub struct FolderKeyError;

impl TryFrom<&str> for FolderKey {
    type Error = FolderKeyError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text.strip_prefix("~/") {
            Some(rest) if !rest.contains('\0') => Ok(Self::Home(HomeRelative(rest.to_owned()))),
            Some(_) => Err(FolderKeyError),
            None => AbsolutePath::try_from(text)
                .map(Self::Absolute)
                .map_err(|_not_absolute| FolderKeyError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FolderKey, FolderKeyError};

    #[test]
    fn a_folder_key_is_absolute_or_starts_with_tilde_slash() {
        assert!(matches!(
            FolderKey::try_from("/work/client-a"),
            Ok(FolderKey::Absolute(_))
        ));
        match FolderKey::try_from("~/work/client-a") {
            Ok(FolderKey::Home(rest)) => assert_eq!(rest.as_str(), "work/client-a"),
            other => panic!("not a home folder: {other:?}"),
        }
        for bad in ["", "~", "work/a", "./a", "~user/a", "~\\a", "~/a\0"] {
            assert_eq!(FolderKey::try_from(bad), Err(FolderKeyError), "{bad:?}");
        }
    }
}
