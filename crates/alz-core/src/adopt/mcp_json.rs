//! `.mcp.json` (spec §4.3): `mcp.yaml` and the manifest's `mcp.enable`. A layer never stores a
//! value of the environment, so an `env` entry is adopted only as a reference to a variable of the
//! same name (`"NAME": "${NAME}"`), which becomes `env_from`. Any other value is a problem, and the
//! problem never prints it.

use crate::diagnostic::Message;
use crate::json::{JsonObject, JsonValue};
use crate::render::yaml::EmitValue;
use crate::untrusted_text::QuotedKey;

use super::draft::Draft;
use super::settings_json::{as_text, object};
use super::values::{from_json, map, text, texts};

const FILE: &str = ".mcp.json";

/// The fields of a server that `mcp.yaml` can hold.
const SERVER_FIELDS: [&str; 5] = ["type", "command", "args", "env", "url"];

/// Plans the adoption of the `.mcp.json` whose bytes are `bytes`.
pub(super) fn adopt(bytes: &[u8], draft: &mut Draft<'_>) {
    let Some(json) = object(bytes, FILE, draft) else {
        return;
    };
    let servers = match json.get("mcpServers") {
        Some(JsonValue::Object(servers)) => Some(servers),
        _ => None,
    };
    let mut out = Vec::<(String, EmitValue)>::new();
    for (id, server) in servers.into_iter().flat_map(JsonObject::iter) {
        let JsonValue::Object(server) = server else {
            continue;
        };
        let extra: Vec<&str> = server
            .iter()
            .map(|(key, _)| key)
            .filter(|key| !SERVER_FIELDS.contains(key))
            .collect();
        if !extra.is_empty() {
            draft.problem(
                FILE,
                Message::new("server ")
                    .then(QuotedKey::from_untrusted(id))
                    .then(" uses fields that mcp.yaml does not support yet"),
            );
            continue;
        }
        let env_from = env_from(id, server.get("env"), draft);
        let transport = server.get("type").and_then(as_text).unwrap_or("stdio");
        let fields = [
            Some(("transport", text(transport))),
            field("url", server.get("url"), id, draft),
            field("command", server.get("command"), id, draft),
            field("args", server.get("args"), id, draft),
            (!env_from.is_empty())
                .then(|| ("env_from", texts(env_from.iter().map(String::as_str)))),
        ];
        out.push((id.to_owned(), map(fields.into_iter().flatten())));
        draft.mcp_enable.push(id.to_owned());
    }
    let mut outputs = Vec::<String>::new();
    if !out.is_empty() {
        draft.merge_yaml(FILE, "mcp.yaml", map(out));
        outputs.push("mcp.yaml".to_owned());
    }
    if !draft.mcp_enable.is_empty() {
        outputs.push("layer.yaml".to_owned());
    }
    draft.converted(FILE, &outputs);
}

/// The variables a server's `env` names, each as `"NAME": "${NAME}"`; any other entry is a problem.
fn env_from(id: &str, env: Option<&JsonValue>, draft: &mut Draft<'_>) -> Vec<String> {
    let Some(JsonValue::Object(env)) = env else {
        return Vec::new();
    };
    env.iter()
        .filter_map(|(name, value)| {
            if as_text(value).is_some_and(|value| value == format!("${{{name}}}")) {
                Some(name.to_owned())
            } else {
                draft.problem(
                    FILE,
                    Message::new("server ")
                        .then(QuotedKey::from_untrusted(id))
                        .then(" sets ")
                        .then(QuotedKey::from_untrusted(name))
                        .then(" to a value; layers never store values. Write the variable as a reference to itself and export it"),
                );
                None
            }
        })
        .collect()
}

/// The entry `name: value` when the server has the field, or a problem when it cannot be written.
fn field(
    name: &'static str,
    value: Option<&JsonValue>,
    id: &str,
    draft: &mut Draft<'_>,
) -> Option<(&'static str, EmitValue)> {
    value.and_then(|value| match from_json(value) {
        Ok(value) => Some((name, value)),
        Err(why) => {
            draft.problem(
                FILE,
                Message::new("server ")
                    .then(QuotedKey::from_untrusted(id))
                    .then(": ")
                    .then(name)
                    .then(" holds ")
                    .then(crate::untrusted_text::Stripped::from_untrusted(
                        why.describe(),
                    ))
                    .then(", which a layer file cannot hold"),
            );
            None
        }
    })
}
