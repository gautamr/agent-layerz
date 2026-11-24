//! The `--stack` option of `alz init` (spec §10.3).

use crate::scaffold::error::{ScaffoldError, StackSourceProblem};
use crate::stack::{GitRef, Source, StackEntry};

/// A `--stack` value as a stack entry. A value that starts with `github:` or `git:` is a source as
/// written. Any other is a `path:` source, with or without the `path:`, and `<dir>@<ref>` pins a
/// ref when the text after the last `@` holds no `/`.
///
/// # Errors
///
/// [`ScaffoldError::ControlCharacter`] for a value that would break the line it is written on,
/// and [`ScaffoldError::StackSource`] for a value that is not a source or whose pin is not a ref.
pub fn stack_entry(value: &str) -> Result<StackEntry, ScaffoldError> {
    if value.chars().any(char::is_control) {
        return Err(ScaffoldError::ControlCharacter);
    }
    let problem = |error| ScaffoldError::StackSource(StackSourceProblem::Source(error));
    if value.starts_with("github:") || value.starts_with("git:") {
        return Source::try_from(value)
            .map(|source| StackEntry::new(source, None))
            .map_err(problem);
    }
    let path = value.strip_prefix("path:").unwrap_or(value);
    let (directory, pin) = path
        .rsplit_once('@')
        .filter(|(directory, pin)| !directory.is_empty() && !pin.contains('/'))
        .map_or((path, None), |(directory, pin)| (directory, Some(pin)));
    let source = Source::try_from(format!("path:{directory}").as_str()).map_err(problem)?;
    let pin = pin
        .map(|text| {
            GitRef::try_from(text)
                .map_err(|_not_a_ref| ScaffoldError::StackSource(StackSourceProblem::Pin))
        })
        .transpose()?;
    Ok(StackEntry::new(source, pin))
}

#[cfg(test)]
mod tests {
    use super::stack_entry;
    use crate::scaffold::{ScaffoldError, StackSourceProblem};
    use crate::stack::SourceError;

    fn written(value: &str) -> (String, Option<String>) {
        let entry = stack_entry(value).expect("an entry");
        (
            entry.source().to_string(),
            entry.pin().map(ToString::to_string),
        )
    }

    #[test]
    fn a_bare_path_means_a_path_source() {
        assert_eq!(written("../personal"), ("path:../personal".into(), None));
        assert_eq!(written("path:./x"), ("path:./x".into(), None));
    }

    #[test]
    fn the_text_after_the_last_at_pins_a_ref_when_it_holds_no_slash() {
        assert_eq!(
            written("path:../hub/layer@layer-v1.0.0"),
            ("path:../hub/layer".into(), Some("layer-v1.0.0".into()))
        );
        assert_eq!(
            written("../a@b/c"),
            ("path:../a@b/c".into(), None),
            "a slash after the @ means the @ is part of the path"
        );
    }

    #[test]
    fn a_github_or_git_value_is_a_source_as_written() {
        assert_eq!(
            written("github:me/agent-layerz-personal@v1.2.0"),
            ("github:me/agent-layerz-personal@v1.2.0".into(), None)
        );
        assert_eq!(
            stack_entry("github:me/repo"),
            Err(ScaffoldError::StackSource(StackSourceProblem::Source(
                SourceError::MissingRef
            )))
        );
    }

    #[test]
    fn a_pin_that_is_not_a_ref_and_a_control_character_are_refused() {
        assert_eq!(
            stack_entry("../a@-x"),
            Err(ScaffoldError::StackSource(StackSourceProblem::Pin))
        );
        assert_eq!(stack_entry("../a\nb"), Err(ScaffoldError::ControlCharacter));
    }
}
