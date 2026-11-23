//! The text of a layer's `layer.yaml` that the leak guard checks: the file without its
//! `forbid_patterns` field, because the patterns are written there (spec §9.2).

use crate::lint::parse_document;
use crate::yaml::{Node, Value};

/// The field whose lines are left out.
const FIELD: &str = "forbid_patterns";

/// The text of `bytes`, read lossily, without the lines of the `forbid_patterns` field: from the
/// line of its key up to the line before the next key. Comment lines in that span stay, because
/// a comment is not a pattern. When the key does not stand on lines of its own (a mapping written
/// on one line) nothing is left out, so the patterns are then matched against themselves and
/// refused, which errs on the side of the guard.
pub(super) fn without_forbid_patterns(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    match field_lines(bytes) {
        Some((first, after)) => kept(&text, first, after),
        None => text.into_owned(),
    }
}

/// The lines of `text` outside `first..after` (counted from 1), and the comment lines inside.
fn kept(text: &str, first: u32, after: u32) -> String {
    text.split('\n')
        .zip(1_u32..)
        .filter(|(line, number)| {
            *number < first || *number >= after || line.trim_start().starts_with('#')
        })
        .map(|(line, _)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The line of the key and the line of the next one (`u32::MAX` for the last), when the key stands
/// alone on its line among the keys of the document.
fn field_lines(bytes: &[u8]) -> Option<(u32, u32)> {
    let document = parse_document(bytes).ok()?;
    let Value::Map(mapping) = document.value() else {
        return None;
    };
    let keys: Vec<(Option<&str>, u32)> = mapping
        .entries()
        .map(|(key, _)| (key_text(key), key.at().line()))
        .collect();
    let index = keys.iter().position(|(name, _)| *name == Some(FIELD))?;
    let line = keys.get(index)?.1;
    let before = index
        .checked_sub(1)
        .and_then(|i| keys.get(i))
        .map(|key| key.1);
    let after = keys.get(index.checked_add(1)?).map(|key| key.1);
    let alone =
        before.is_none_or(|previous| previous < line) && after.is_none_or(|next| next > line);
    alone.then_some((line, after.unwrap_or(u32::MAX)))
}

/// The text of a key, `None` for a key that is not text.
fn key_text(key: &Node) -> Option<&str> {
    match key.value() {
        Value::Text(text) => Some(text.as_str()),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::without_forbid_patterns;

    fn text(yaml: &str) -> String {
        without_forbid_patterns(yaml.as_bytes())
    }

    #[test]
    fn the_field_is_left_out_and_its_neighbours_stay() {
        let yaml = "name: a\nforbid_patterns:\n  - acme\n  - \"x y\"\nversion: 1\n";
        assert_eq!(text(yaml), "name: a\nversion: 1\n");
    }

    #[test]
    fn the_last_field_runs_to_the_end() {
        assert_eq!(text("name: a\nforbid_patterns: [acme]\n"), "name: a");
        assert_eq!(text("name: a\nforbid_patterns: [acme]"), "name: a");
    }

    #[test]
    fn a_comment_between_the_patterns_and_the_next_key_stays() {
        let yaml = "forbid_patterns:\n  - acme\n# notes for acme\nname: a\n";
        assert_eq!(text(yaml), "# notes for acme\nname: a\n");
    }

    #[test]
    fn a_mapping_on_one_line_is_kept_whole() {
        let yaml = "{name: a, forbid_patterns: [acme]}\n";
        assert_eq!(text(yaml), yaml);
    }

    #[test]
    fn a_file_without_the_field_or_that_is_not_a_mapping_is_kept_whole() {
        assert_eq!(text("name: a\n"), "name: a\n");
        assert_eq!(text("- a\n"), "- a\n");
        assert_eq!(text("a: [\n"), "a: [\n");
    }
}
