//! `.claude/settings.json` (spec §4.3): `permissions.yaml`, `hooks/hooks.yaml`, `settings.yaml`,
//! the `models.yaml` default and the manifest's `workspace.add_dirs`. A part that would loosen
//! security if it were dropped is a problem.

use crate::diagnostic::Message;
use crate::json::{JsonNumber, JsonObject, JsonValue, parse};
use crate::render::yaml::EmitValue;
use crate::untrusted_text::QuotedKey;

use super::agent::model_alias;
use super::draft::Draft;
use super::hooks::adopt_hooks;
use super::names;
use super::values::{from_json, map, text};

const FILE: &str = ".claude/settings.json";

/// Keys the manifest's `mcp.enable` replaces: dropped with a note.
const MCP_SWITCHES: [&str; 3] = [
    "enabledMcpjsonServers",
    "disabledMcpjsonServers",
    "enableAllProjectMcpServers",
];

/// Plans the adoption of the settings file whose bytes are `bytes`. `scripts` are the hook
/// scripts that move to the layer, as (repository path, path from `hooks/`).
pub(super) fn adopt(bytes: &[u8], scripts: &[(String, String)], draft: &mut Draft<'_>) {
    let Some(json) = object(bytes, FILE, draft) else {
        return;
    };
    let mut outputs = Vec::<String>::new();
    let mut settings = Vec::<(String, EmitValue)>::new();
    for (key, value) in json.iter() {
        match key {
            "permissions" => adopt_permissions(value, draft, &mut outputs),
            "hooks" => adopt_hooks(value, scripts, draft, &mut outputs),
            "model" => match value {
                JsonValue::Text(model) => {
                    if let Some(alias) = model_alias(model, FILE, draft) {
                        draft.merge_yaml(FILE, "models.yaml", map([("default", text(&alias))]));
                        push_once(&mut outputs, "models.yaml");
                    }
                }
                JsonValue::Null
                | JsonValue::Bool(_)
                | JsonValue::Number(_)
                | JsonValue::Array(_)
                | JsonValue::Object(_) => draft.problem(FILE, Message::new("model is not text")),
            },
            switch if MCP_SWITCHES.contains(&switch) => draft.note(
                FILE,
                Message::new("dropped ")
                    .then(QuotedKey::from_untrusted(switch))
                    .then("; the manifest's mcp.enable decides which servers run"),
            ),
            "disableAllHooks" => draft.problem(
                FILE,
                Message::new(
                    "'disableAllHooks' has no layered form; remove it or disable hooks by id",
                ),
            ),
            other => match from_json(value) {
                Ok(value) => settings.push((other.to_owned(), value)),
                Err(why) => draft.unwritable(FILE, other, why.describe()),
            },
        }
    }
    if !settings.is_empty() {
        draft.merge_yaml(FILE, "settings.yaml", map([("claude", map(settings))]));
        push_once(&mut outputs, "settings.yaml");
    }
    draft.converted(FILE, &outputs);
}

/// The JSON object of a native file, or a problem.
pub(super) fn object(bytes: &[u8], file: &str, draft: &mut Draft<'_>) -> Option<JsonObject> {
    if let Ok(JsonValue::Object(object)) = parse(bytes) {
        Some(object)
    } else {
        draft.problem(file, Message::new("is not a JSON object"));
        None
    }
}

pub(super) fn push_once(outputs: &mut Vec<String>, path: &str) {
    if !outputs.iter().any(|seen| seen == path) {
        outputs.push(path.to_owned());
    }
}

fn adopt_permissions(value: &JsonValue, draft: &mut Draft<'_>, outputs: &mut Vec<String>) {
    let JsonValue::Object(permissions) = value else {
        draft.problem(FILE, Message::new("permissions must be an object"));
        return;
    };
    let mut out = Vec::<(String, EmitValue)>::new();
    for (key, rules) in permissions.iter() {
        match key {
            "allow" | "ask" | "deny" => {
                let list: Vec<&str> = match rules {
                    JsonValue::Array(items) => items.iter().filter_map(as_text).collect(),
                    JsonValue::Null
                    | JsonValue::Bool(_)
                    | JsonValue::Number(_)
                    | JsonValue::Text(_)
                    | JsonValue::Object(_) => Vec::new(),
                };
                let neutral: Vec<String> = list
                    .into_iter()
                    .filter_map(|rule| {
                        let neutral = names::rule(rule);
                        if neutral.is_none() {
                            draft.problem(
                                &format!("{FILE}.{key}"),
                                Message::new(QuotedKey::from_untrusted(rule))
                                    .then(" uses a tool agent-layerz does not know"),
                            );
                        }
                        neutral
                    })
                    .collect();
                out.push((
                    key.to_owned(),
                    EmitValue::Seq(neutral.iter().map(|rule| text(rule)).collect()),
                ));
            }
            "defaultMode" => match rules {
                JsonValue::Text(mode) => match names::mode(mode) {
                    Some(mode) => out.push(("default_mode".to_owned(), text(mode.as_str()))),
                    None => draft.problem(
                        FILE,
                        Message::new("unknown defaultMode ").then(QuotedKey::from_untrusted(mode)),
                    ),
                },
                JsonValue::Null
                | JsonValue::Bool(_)
                | JsonValue::Number(_)
                | JsonValue::Array(_)
                | JsonValue::Object(_) => {
                    draft.problem(FILE, Message::new("defaultMode is not text"));
                }
            },
            "additionalDirectories" => {
                if let JsonValue::Array(dirs) = rules {
                    for dir in dirs.iter().filter_map(as_text) {
                        if dir.starts_with('/') || dir.starts_with('~') {
                            draft.problem(
                                FILE,
                                Message::new("additional directory ")
                                    .then(QuotedKey::from_untrusted(dir))
                                    .then(" must be relative to the repo"),
                            );
                        } else {
                            draft.add_dirs.push(dir.to_owned());
                        }
                    }
                }
                push_once(outputs, "layer.yaml");
            }
            other => draft.problem(
                FILE,
                Message::new("permissions.")
                    .then(QuotedKey::from_untrusted(other))
                    .then(" has no layered form yet; move it by hand"),
            ),
        }
    }
    if !out.is_empty() {
        draft.merge_yaml(FILE, "permissions.yaml", map(out));
        push_once(outputs, "permissions.yaml");
    }
}

pub(super) const fn as_text(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::Text(text) => Some(text.as_str()),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Array(_)
        | JsonValue::Object(_) => None,
    }
}

/// A whole number of a JSON value that fits `i64`.
pub(super) fn as_whole(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(JsonNumber::Int(number)) => i64::try_from(*number).ok(),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(JsonNumber::Float(_))
        | JsonValue::Text(_)
        | JsonValue::Array(_)
        | JsonValue::Object(_) => None,
    }
}
