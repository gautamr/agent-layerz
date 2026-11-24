//! `.claude/agents/<name>.md` (spec §4.3): a hand-written agent becomes a neutral agent of the repo
//! layer. Native-only fields go under `claude:`, a concrete model becomes a `models.yaml` alias,
//! and an agent that a more general layer defines becomes a `mode: replace` overlay, so the repo's
//! version keeps winning as it did before.

use crate::diagnostic::Message;
use crate::markdown::MarkdownDoc;
use crate::path::SafeRelPath;
use crate::render::yaml::{EmitValue, emit};
use crate::target::TargetName;
use crate::untrusted_text::{QuotedKey, Stripped};
use crate::yaml::{Node, Value};

use super::NativeFile;
use super::draft::Draft;
use super::names;
use super::values::{from_agent_node, map, text, texts};

/// The fields that map to a neutral field one to one.
const SAME: [&str; 4] = ["description", "skills", "isolation", "memory"];

/// What one frontmatter entry becomes.
enum Field {
    /// An entry of the neutral agent.
    Neutral(&'static str, EmitValue),
    /// An entry that only Claude knows, kept under `claude:`.
    Native(String, EmitValue),
    /// An entry that is dropped.
    Dropped,
}

/// Plans the adoption of the agent at `path`, a file `.claude/agents/<name>.md`.
pub(super) fn adopt(path: &SafeRelPath, file: &NativeFile, draft: &mut Draft<'_>) {
    let from = path.as_str();
    let name = path
        .as_str()
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".md"))
        .unwrap_or_default()
        .to_owned();
    let doc = match MarkdownDoc::parse(file.bytes()) {
        Ok(doc) => doc,
        Err(error) => {
            draft.problem(
                from,
                Message::new(Stripped::from_untrusted(&error.to_string())),
            );
            return;
        }
    };
    let overlay = draft.ctx.defines_agent(&name);
    let (front, body) = doc.into_parts();
    let head = if overlay {
        vec![
            ("extends".to_owned(), text(&name)),
            ("mode".to_owned(), text("replace")),
        ]
    } else {
        vec![("name".to_owned(), text(&name))]
    };
    let mut changed = overlay;
    let mut neutral = head;
    let mut native = Vec::new();
    for (key, value) in front.mapping().entries() {
        let Value::Text(key) = key.value() else {
            draft.problem(from, Message::new("has a frontmatter key that is not text"));
            continue;
        };
        if key == "name" {
            continue;
        }
        match field(key, value, from, draft) {
            Field::Neutral(key, value) => neutral.push((key.to_owned(), value)),
            Field::Native(key, value) => native.push((key, value)),
            Field::Dropped => {}
        }
        changed |= !SAME.contains(&key.as_str());
    }
    let to = if overlay {
        format!("agents/{name}.overlay.md")
    } else {
        format!("agents/{name}.md")
    };
    if !changed {
        draft.move_file(path, file, &to);
        return;
    }
    if overlay {
        draft.note(
            from,
            Message::new("a more general layer defines ")
                .then(QuotedKey::from_untrusted(&name))
                .then("; adopted as a mode: replace overlay"),
        );
    }
    let claude = (!native.is_empty()).then(|| ("claude".to_owned(), map(native)));
    let front = map(neutral.into_iter().chain(claude));
    draft.write_text(
        from,
        &to,
        format!("---\n{}---\n{}", emit(&front), body.as_str()),
    );
    draft.converted(from, &[to]);
}

/// What the frontmatter entry `key: value` becomes.
fn field(key: &str, value: &Node, from: &str, draft: &mut Draft<'_>) -> Field {
    match key {
        "tools" => match tools(value) {
            Ok(tools) => Field::Neutral("tools", texts(tools.iter().map(String::as_str))),
            Err(entry) => {
                draft.problem(
                    from,
                    Message::new("tools entry ")
                        .then(QuotedKey::from_untrusted(&entry))
                        .then(" has no neutral form; set it by hand"),
                );
                Field::Dropped
            }
        },
        "model" => match value.value() {
            Value::Text(model) if model == "inherit" => Field::Dropped,
            Value::Text(model) => model_alias(model, from, draft).map_or(Field::Dropped, |alias| {
                Field::Neutral("model", text(&alias))
            }),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Seq(_)
            | Value::Map(_) => {
                draft.problem(from, Message::new("model is not text"));
                Field::Dropped
            }
        },
        "permissionMode" => match value.value() {
            Value::Text(mode) => names::mode(mode).map_or_else(
                || {
                    draft.problem(
                        from,
                        Message::new("unknown permissionMode ")
                            .then(QuotedKey::from_untrusted(mode)),
                    );
                    Field::Dropped
                },
                |mode| Field::Neutral("permission_mode", text(mode.as_str())),
            ),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Seq(_)
            | Value::Map(_) => {
                draft.problem(from, Message::new("permissionMode is not text"));
                Field::Dropped
            }
        },
        _ => match from_agent_node(value) {
            Ok(value) => SAME.iter().find(|same| **same == key).map_or_else(
                || Field::Native(key.to_owned(), value.clone()),
                |same| Field::Neutral(same, value.clone()),
            ),
            Err(why) => {
                draft.unwritable(from, key, why.describe());
                Field::Dropped
            }
        },
    }
}

/// `Read, Grep` or a list as neutral tools; `Err` holds the entry that has no neutral form.
fn tools(value: &Node) -> Result<Vec<String>, String> {
    let entries: Vec<String> = match value.value() {
        Value::Seq(items) => items.iter().map(entry_text).collect(),
        Value::Text(line) => line
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect(),
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Map(_) => {
            vec![String::new()]
        }
    };
    entries
        .into_iter()
        .map(|entry| names::tool(&entry).ok_or(entry))
        .collect()
}

fn entry_text(node: &Node) -> String {
    match node.value() {
        Value::Text(text) => text.clone(),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => String::new(),
    }
}

/// The alias of a concrete model (spec §7.8): one the stack has for it, or one named after the
/// model, which `models.yaml` gets. An alias a general layer owns is never redefined.
pub(super) fn model_alias(model: &str, from: &str, draft: &mut Draft<'_>) -> Option<String> {
    let models = draft.ctx.merged.models();
    let existing = models
        .aliases_in_definition_order()
        .find(|alias| {
            models
                .model(alias, TargetName::Claude)
                .is_some_and(|concrete| concrete.as_str() == model)
        })
        .map(|alias| alias.as_str().to_owned());
    if existing.is_some() {
        return existing;
    }
    let taken = models.aliases().keys().any(|alias| alias.as_str() == model);
    let name = if taken {
        format!("claude-{model}")
    } else {
        model.to_owned()
    };
    draft.merge_yaml(
        from,
        "models.yaml",
        map([(
            "aliases",
            map([(name.clone(), map([("claude", text(model))]))]),
        )]),
    );
    Some(name)
}
