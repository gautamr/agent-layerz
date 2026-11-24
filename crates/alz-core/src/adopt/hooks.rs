//! The `hooks` of `.claude/settings.json` (spec §4.3, §7.5): each command hook becomes an entry of
//! `hooks/hooks.yaml`. A hook of another type, or with a field this conversion does not know, is a
//! problem: dropping it would change what runs.

use crate::diagnostic::Message;
use crate::json::JsonValue;
use crate::render::yaml::EmitValue;
use crate::untrusted_text::Stripped;

use super::draft::Draft;
use super::names::{self, HOOKS_DIR, PROJECT_DIR_VAR};
use super::settings_json::{as_text, as_whole, push_once};
use super::values::{map, text, texts};

const FILE: &str = ".claude/settings.json";

/// The fields of a command hook that adoption converts.
const HANDLER_FIELDS: [&str; 4] = ["type", "command", "timeout", "onFailure"];

/// The extensions of a script whose name gives a hook its id.
const SCRIPT_EXTENSIONS: [&str; 8] = ["sh", "bash", "js", "mjs", "cjs", "ts", "py", "rb"];

/// Plans `hooks/hooks.yaml` from the settings' `hooks`. `scripts` are the hook scripts moved to
/// the layer, as (repository path, path from `hooks/`).
pub(super) fn adopt_hooks(
    value: &JsonValue,
    scripts: &[(String, String)],
    draft: &mut Draft<'_>,
    outputs: &mut Vec<String>,
) {
    let JsonValue::Object(events) = value else {
        draft.problem(FILE, Message::new("hooks must be an object"));
        return;
    };
    let mut hooks = Vec::<EmitValue>::new();
    let mut ids = Vec::<String>::new();
    for (event, groups) in events.iter() {
        let JsonValue::Array(groups) = groups else {
            continue;
        };
        for group in groups {
            let (matcher, handlers) = match group {
                JsonValue::Object(group) => (
                    group.get("matcher").and_then(as_text),
                    match group.get("hooks") {
                        Some(JsonValue::Array(handlers)) => handlers.as_slice(),
                        _ => &[],
                    },
                ),
                JsonValue::Null
                | JsonValue::Bool(_)
                | JsonValue::Number(_)
                | JsonValue::Text(_)
                | JsonValue::Array(_) => (None, &[][..]),
            };
            for handler in handlers {
                match command_hook(handler) {
                    Some(hook) => {
                        let entry = hook_entry(event, matcher, &hook, scripts, &mut ids, draft);
                        hooks.push(entry);
                    }
                    None => draft.problem(
                        FILE,
                        Message::new("hooks.")
                            .then(Stripped::from_untrusted(event))
                            .then(": only command hooks with type, command, timeout, onFailure can be adopted; move this one by hand"),
                    ),
                }
            }
        }
    }
    if !hooks.is_empty() {
        draft.merge_yaml(FILE, "hooks/hooks.yaml", EmitValue::Seq(hooks));
        push_once(outputs, "hooks/hooks.yaml");
    }
}

/// A command hook of the settings, with the fields adoption converts.
struct CommandHook<'a> {
    command: &'a str,
    timeout: Option<&'a JsonValue>,
    block: bool,
}

/// The command hook `handler` is, or `None` when it is another type or holds another field.
fn command_hook(handler: &JsonValue) -> Option<CommandHook<'_>> {
    let JsonValue::Object(handler) = handler else {
        return None;
    };
    let known = handler.iter().all(|(key, _)| HANDLER_FIELDS.contains(&key));
    let is_command = handler.get("type").and_then(as_text) == Some("command");
    let command = handler.get("command").and_then(as_text)?;
    (known && is_command).then(|| CommandHook {
        command,
        timeout: handler.get("timeout"),
        block: handler.get("onFailure").and_then(as_text) == Some("block"),
    })
}

/// The neutral entry of one hook: a neutral event where one exists, otherwise the native
/// `claude:<Event>` with its matcher.
fn hook_entry(
    event: &str,
    matcher: Option<&str>,
    hook: &CommandHook<'_>,
    scripts: &[(String, String)],
    ids: &mut Vec<String>,
    draft: &mut Draft<'_>,
) -> EmitValue {
    let all = matcher.is_none_or(|matcher| matcher.is_empty() || matcher == "*");
    let tools: Vec<Option<String>> = matcher.filter(|_| !all).map_or_else(Vec::new, |matcher| {
        matcher
            .split('|')
            .map(|tool| names::tool(tool.trim()))
            .collect()
    });
    let neutral = names::event(event)
        .filter(|event| all || (names::is_tool_event(*event) && tools.iter().all(Option::is_some)));
    let id = unique_id(&hook_id(hook.command, event), ids);
    let when: Vec<(String, EmitValue)> = match neutral {
        Some(neutral) => {
            let tools: Vec<&str> = tools.iter().flatten().map(String::as_str).collect();
            [("event".to_owned(), text(neutral.as_str()))]
                .into_iter()
                .chain((!tools.is_empty()).then(|| {
                    (
                        "match".to_owned(),
                        map([("tools", texts(tools.iter().copied()))]),
                    )
                }))
                .collect()
        }
        None => [("event".to_owned(), text(&format!("claude:{event}")))]
            .into_iter()
            .chain(
                matcher
                    .filter(|_| !all)
                    .map(|matcher| ("matcher".to_owned(), text(matcher))),
            )
            .collect(),
    };
    let timeout = hook
        .timeout
        .filter(|value| matches!(value, JsonValue::Number(_)))
        .and_then(|number| {
            let seconds = as_whole(number);
            if seconds.is_none() {
                draft.unwritable(
                    FILE,
                    "timeout",
                    "a number that is not a whole number of seconds",
                );
            }
            seconds.map(EmitValue::Int)
        });
    map([("id".to_owned(), text(&id))]
        .into_iter()
        .chain(when)
        .chain([action(hook.command, scripts, draft.ctx.layer_name.as_str())])
        .chain(timeout.map(|timeout| ("timeout".to_owned(), timeout)))
        .chain([(
            "on_fail".to_owned(),
            text(if hook.block { "deny" } else { "warn" }),
        )]))
}

/// A command that only runs a hand-written `.claude/hooks/` script becomes `run:` on its layer
/// copy; one that mentions it points at the layer's copy.
fn action(command: &str, scripts: &[(String, String)], layer: &str) -> (String, EmitValue) {
    let neutral = names::neutral_command(command);
    let bare = bare_path(&neutral);
    scripts
        .iter()
        .find_map(|(path, script)| {
            if bare == *path {
                Some(("run".to_owned(), text(script)))
            } else if neutral.contains(path.as_str()) {
                Some((
                    "command".to_owned(),
                    text(&neutral.replacen(
                        path.as_str(),
                        &format!("{HOOKS_DIR}/{layer}/{script}"),
                        1,
                    )),
                ))
            } else {
                None
            }
        })
        .unwrap_or_else(|| ("command".to_owned(), text(&neutral)))
}

/// The command without one quote at each end and without a leading `${PROJECT_DIR}/`.
fn bare_path(command: &str) -> &str {
    let trimmed = command.trim();
    let unquoted = trimmed.strip_prefix('"').unwrap_or(trimmed);
    let unquoted = unquoted.strip_suffix('"').unwrap_or(unquoted);
    let rest = unquoted.strip_prefix('"').unwrap_or(unquoted);
    rest.strip_prefix(PROJECT_DIR_VAR)
        .map(|after| after.strip_prefix('"').unwrap_or(after))
        .and_then(|after| after.strip_prefix('/'))
        .unwrap_or(unquoted)
}

/// Named after the script the command runs (`session-brief.sh` is `session-brief`), else the
/// event, in lower-case words joined by `-`.
fn hook_id(command: &str, event: &str) -> String {
    let word = script_name(command).unwrap_or(event);
    let spaced: String = word
        .chars()
        .fold((String::new(), None::<char>), |(mut text, before), c| {
            if c.is_ascii_uppercase()
                && before.is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            {
                text.push('-');
            }
            text.push(c.to_ascii_lowercase());
            (text, Some(c))
        })
        .0;
    spaced
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// The name before the first `.<script extension>` in a command: the `[\w-]+` run before it.
fn script_name(command: &str) -> Option<&str> {
    command.match_indices('.').find_map(|(dot, _)| {
        let after = command.get(dot + 1..)?;
        SCRIPT_EXTENSIONS.iter().find(|extension| {
            after.strip_prefix(**extension).is_some_and(|rest| {
                !rest
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            })
        })?;
        let before = command.get(..dot)?;
        let start = before
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .last()
            .map(|(at, _)| at)?;
        before.get(start..)
    })
}

/// `base`, or `base-2`, `base-3`, … when an earlier hook has it. An empty base is `hook`.
fn unique_id(base: &str, ids: &mut Vec<String>) -> String {
    let first = if base.is_empty() { "hook" } else { base };
    let id = std::iter::once(first.to_owned())
        .chain((2_u32..=u32::MAX).map(|n| format!("{base}-{n}")))
        .find(|candidate| !ids.contains(candidate))
        .unwrap_or_else(|| first.to_owned());
    ids.push(id.clone());
    id
}
