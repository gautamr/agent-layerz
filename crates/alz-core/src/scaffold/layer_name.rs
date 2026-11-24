//! The name of the layer `alz init` creates (spec §10.3).

use crate::layer::LayerName;
use crate::scaffold::error::ScaffoldError;
use crate::untrusted_text::Stripped;

/// The name `--name` gave, or else the name of the folder: in lower case, each run of characters
/// other than `a` to `z` and `0` to `9` replaced by `-`, and `-` trimmed at both ends.
///
/// # Errors
///
/// [`ScaffoldError::NotALayerName`] when the result is not a layer name (§5.1).
pub fn layer_name(given: Option<&str>, folder: &str) -> Result<LayerName, ScaffoldError> {
    let text = given.map_or_else(|| name_of_folder(folder), str::to_owned);
    LayerName::try_from(text.as_str())
        .map_err(|_not_a_name| ScaffoldError::NotALayerName(Stripped::from_untrusted(&text)))
}

/// The folder's name as a layer name would read: runs of other characters become one `-`.
fn name_of_folder(folder: &str) -> String {
    folder
        .to_lowercase()
        .split(|character: char| !(character.is_ascii_lowercase() || character.is_ascii_digit()))
        .filter(|run| !run.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::layer_name;
    use crate::scaffold::ScaffoldError;

    #[test]
    fn a_folder_name_becomes_a_layer_name() {
        for (folder, name) in [
            ("My Repo", "my-repo"),
            ("--new_repo--", "new-repo"),
            ("repo2", "repo2"),
            ("a..b", "a-b"),
        ] {
            assert_eq!(
                layer_name(None, folder).map(|n| n.as_str().to_owned()),
                Ok(name.to_owned()),
                "{folder}"
            );
        }
    }

    #[test]
    fn a_folder_with_no_usable_character_has_no_name() {
        assert!(matches!(
            layer_name(None, "___"),
            Err(ScaffoldError::NotALayerName(_))
        ));
    }

    #[test]
    fn a_given_name_is_checked_and_not_changed() {
        assert!(layer_name(Some("a.b-c"), "x").is_ok());
        assert!(matches!(
            layer_name(Some("Bad Name"), "x"),
            Err(ScaffoldError::NotALayerName(_))
        ));
    }
}
