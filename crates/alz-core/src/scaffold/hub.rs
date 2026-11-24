//! The hub of `alz init --hub` (spec §10.3): its client layer, pinned to its newest tag.

use std::ffi::OsStr;

use crate::scaffold::error::ScaffoldError;
use crate::stack::{GitRef, Source, StackEntry};

/// The folder `--hub` names, as it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hub(String);

impl Hub {
    /// The hub folder as written.
    ///
    /// # Errors
    ///
    /// [`ScaffoldError::ControlCharacter`] for text that would break the line of `layer.yaml` it
    /// is written on.
    pub fn new(given: &str) -> Result<Self, ScaffoldError> {
        if given.is_empty() || given.chars().any(char::is_control) {
            Err(ScaffoldError::ControlCharacter)
        } else {
            Ok(Self(given.to_owned()))
        }
    }

    /// The folder as written, for `workspace.hub`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The folder of the client layer, `<hub>/layer`, as a path relative to the repository.
    #[must_use]
    pub fn layer_folder(&self) -> String {
        join_layer(&self.0)
    }

    /// The stack entry of the hub's client layer: `path:<hub>/layer`, pinned to `newest_tag`, the
    /// first tag `layer-v*` in version order, when there is one.
    ///
    /// # Errors
    ///
    /// [`ScaffoldError::HubTagNotARef`] when the tag is not a git ref (§5.2); nothing is written
    /// with it.
    pub fn client_entry(&self, newest_tag: Option<&OsStr>) -> Result<StackEntry, ScaffoldError> {
        let pin = newest_tag
            .map(|tag| {
                tag.to_str()
                    .and_then(|text| GitRef::try_from(text).ok())
                    .ok_or(ScaffoldError::HubTagNotARef)
            })
            .transpose()?;
        Source::try_from(format!("path:{}", self.layer_folder()).as_str())
            .map(|source| StackEntry::new(source, pin))
            .map_err(|_empty| ScaffoldError::ControlCharacter)
    }
}

/// `<hub>/layer` joined and normalised as POSIX `path.join` does: empty and `.` parts dropped, a
/// `..` cancelling the part before it unless that is a `..` or there is none (and the path is
/// relative), and `.` for nothing.
fn join_layer(hub: &str) -> String {
    let absolute = hub.starts_with('/');
    let parts = hub
        .split('/')
        .chain(["layer"])
        .filter(|part| !part.is_empty() && *part != ".")
        .fold(Vec::<&str>::new(), |mut parts, part| {
            match (part, parts.last()) {
                ("..", Some(last)) if *last != ".." => {
                    parts.pop();
                }
                ("..", None) if absolute => {}
                _ => parts.push(part),
            }
            parts
        });
    match (absolute, parts.is_empty()) {
        (true, _) => format!("/{}", parts.join("/")),
        (false, true) => ".".to_owned(),
        (false, false) => parts.join("/"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::Hub;
    use crate::scaffold::ScaffoldError;

    fn folder(hub: &str) -> String {
        Hub::new(hub).expect("a hub").layer_folder()
    }

    #[test]
    fn the_client_layer_folder_is_joined_as_a_posix_path() {
        assert_eq!(folder("../hub"), "../hub/layer");
        assert_eq!(folder("./hub/"), "hub/layer");
        assert_eq!(folder("a/../b"), "b/layer");
        assert_eq!(folder("../.."), "../../layer");
        assert_eq!(folder("."), "layer");
        assert_eq!(folder("/abs/hub"), "/abs/hub/layer");
    }

    #[test]
    fn the_newest_tag_pins_the_entry() {
        let hub = Hub::new("../hub").expect("a hub");
        let pinned = hub
            .client_entry(Some(OsStr::new("layer-v0.10.0")))
            .expect("an entry");
        assert_eq!(pinned.source().to_string(), "path:../hub/layer");
        assert_eq!(
            pinned.pin().map(ToString::to_string),
            Some("layer-v0.10.0".to_owned())
        );
        assert_eq!(hub.client_entry(None).expect("an entry").pin(), None);
    }

    #[test]
    fn a_tag_that_is_not_a_git_ref_is_refused_before_anything_is_written() {
        let hub = Hub::new("../hub").expect("a hub");
        for tag in ["layer-v1..2", "layer-v1 x", "layer-v1~"] {
            assert_eq!(
                hub.client_entry(Some(OsStr::new(tag))),
                Err(ScaffoldError::HubTagNotARef),
                "{tag}"
            );
        }
    }

    #[test]
    fn a_hub_with_a_control_character_or_no_text_is_refused() {
        assert_eq!(Hub::new(""), Err(ScaffoldError::ControlCharacter));
        assert_eq!(Hub::new("a\nb"), Err(ScaffoldError::ControlCharacter));
    }
}
