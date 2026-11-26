//! The keys an adapter writes, listed from its structured output (spec §8.0 **Output.**).

use std::collections::BTreeSet;

use alz_adapter::{Content, Frontmatter, JsonValue, Output, TomlValue};

use super::key_path::KeyPath;

/// The keys of one output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenFile {
    /// The path of the file, relative to the repository.
    pub file: String,
    /// The key paths the file holds, each once: every key of every object, table and frontmatter
    /// mapping at any depth, and none for scalars or the items of an array of scalars. A file of
    /// bytes, or Markdown without frontmatter, holds none.
    pub keys: BTreeSet<KeyPath>,
}

/// The keys of every file of `output`, in the order of the files.
///
/// - [`Content::Json`] and [`Content::Toml`]: the keys of the value.
/// - [`Content::Markdown`] with a mapping frontmatter: the keys of the mapping.
/// - [`Content::Markdown`] with a kept frontmatter, the text of a layer file: its top-level keys,
///   read from the lines that start in column 0 with a bare or quoted key and a colon. The kept
///   text is YAML that alz does not parse, so keys below the top level are not listed.
#[must_use]
pub fn written_files(output: &Output) -> Vec<WrittenFile> {
    output
        .files
        .iter()
        .map(|file| WrittenFile {
            file: file.path.clone(),
            keys: keys_of(&file.content),
        })
        .collect()
}

fn keys_of(content: &Content) -> BTreeSet<KeyPath> {
    let top = KeyPath::default();
    match content {
        Content::Json(value) => json_keys(&top, value).into_iter().collect(),
        Content::Toml(value) => toml_keys(&top, value).into_iter().collect(),
        Content::Markdown(markdown) => match &markdown.frontmatter {
            Some(Frontmatter::Mapping(object)) => object
                .entries()
                .flat_map(|(key, value)| {
                    let path = top.key(key);
                    json_keys(&path, value)
                        .into_iter()
                        .chain(Some(path))
                        .collect::<Vec<_>>()
                })
                .collect(),
            Some(Frontmatter::Kept(text)) => text.lines().filter_map(kept_key).collect(),
            None => BTreeSet::new(),
        },
        Content::Bytes(_) => BTreeSet::new(),
    }
}

/// The keys below `prefix` in `value`.
fn json_keys(prefix: &KeyPath, value: &JsonValue) -> Vec<KeyPath> {
    match value {
        JsonValue::Object(object) => object
            .entries()
            .flat_map(|(key, below)| {
                let path = prefix.key(key);
                json_keys(&path, below)
                    .into_iter()
                    .chain(Some(path))
                    .collect::<Vec<_>>()
            })
            .collect(),
        JsonValue::Array(items) => items
            .iter()
            .flat_map(|item| json_keys(&prefix.items(), item))
            .collect(),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Int(_)
        | JsonValue::Float(_)
        | JsonValue::Text(_) => Vec::new(),
    }
}

fn toml_keys(prefix: &KeyPath, value: &TomlValue) -> Vec<KeyPath> {
    match value {
        TomlValue::Table(table) => table
            .iter()
            .flat_map(|(key, below)| {
                let path = prefix.key(key);
                toml_keys(&path, below)
                    .into_iter()
                    .chain(Some(path))
                    .collect::<Vec<_>>()
            })
            .collect(),
        TomlValue::Array(items) => items
            .iter()
            .flat_map(|item| toml_keys(&prefix.items(), item))
            .collect(),
        TomlValue::Text(_)
        | TomlValue::Integer(_)
        | TomlValue::Float(_)
        | TomlValue::Boolean(_) => Vec::new(),
    }
}

/// The top-level key of a line of kept frontmatter, when the line starts one: no indent, a key
/// (bare, or in single or double quotes) and a colon. A comment, a list item and an indented line
/// are not keys.
fn kept_key(line: &str) -> Option<KeyPath> {
    let (key, _) = line.split_once(':')?;
    let name = ['"', '\'']
        .into_iter()
        .find_map(|quote| key.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(key);
    let is_name = !name.is_empty()
        && !name.starts_with(' ')
        && name.chars().all(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | ' ' | '.')
        });
    is_name.then(|| KeyPath::top(name))
}
