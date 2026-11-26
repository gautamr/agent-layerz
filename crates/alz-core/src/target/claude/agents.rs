//! `.claude/agents/<name>.md` (spec §8.1, target-facts §1): an agent that targets Claude, its
//! neutral fields mapped to the native frontmatter, then its native fields in source order.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::item::{ModelAlias, NativeValue, ToolGrant};
use crate::merge::{Merged, MergedAgent, MergedModels, Traced};
use crate::render::yaml::{EmitMap, EmitValue};
use crate::untrusted_text::Stripped;

use super::frontmatter::{all_origins, file, place, sources, text};
use super::{Output, tools};

/// The folder of the agent files.
const AGENTS_DIR: &str = ".claude/agents";

/// The agents that target Claude as files, with the errors and warnings of the mapping.
pub(super) fn render(merged: &Merged) -> Output {
    merged
        .agents()
        .values()
        .filter(|agent| {
            agent
                .value()
                .targets()
                .is_none_or(|targets| targets.contains(crate::target::claude()))
        })
        .map(|agent| one(agent, merged.models()))
        .fold(Output::default(), Output::then)
}

/// What the tools of an agent become.
enum Tools {
    /// No list: the agent inherits every tool, and the field is left out.
    Inherit,
    /// The native names, joined.
    Listed(String),
    /// An empty list, which Claude reads as all tools.
    Empty,
}

fn one(traced: &Traced<MergedAgent>, models: &MergedModels) -> Output {
    let agent = traced.value();
    let at = place(traced.provenance());
    let (tools_field, tool_found) = native_tools(agent, at.as_ref());
    if matches!(tools_field, Tools::Empty) {
        return Output {
            files: Vec::new(),
            diagnostics: tool_found,
        };
    }
    let (model, model_found) = native_model(agent, models, at.as_ref());
    let fixed = [
        Some(("name", text(agent.name().as_str()))),
        Some(("description", text(agent.description().as_str()))),
        match tools_field {
            Tools::Listed(names) => Some(("tools", text(&names))),
            Tools::Inherit | Tools::Empty => None,
        },
        model.map(|value| ("model", text(&value))),
        agent
            .scalars()
            .permission_mode()
            .map(|mode| ("permissionMode", text(tools::permission_mode(*mode)))),
        (!agent.skills().is_empty()).then(|| {
            (
                "skills",
                EmitValue::Seq(
                    agent
                        .skills()
                        .iter()
                        .map(|skill| text(skill.as_str()))
                        .collect(),
                ),
            )
        }),
        agent
            .scalars()
            .isolation()
            .filter(|isolation| matches!(isolation, crate::item::Isolation::Worktree))
            .map(|_| ("isolation", text("worktree"))),
        agent
            .scalars()
            .memory()
            .and_then(|memory| tools::memory(*memory))
            .map(|memory| ("memory", text(memory))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value));
    let found = tool_found.merge(model_found);
    match native_fields(agent) {
        Ok(native) => file(
            &format!("{AGENTS_DIR}/{}.md", agent.name()),
            sources(all_origins(traced.provenance())),
            fixed.chain(native).collect(),
            &agent.body().compose(),
            found,
        ),
        Err(key) => Output {
            files: Vec::new(),
            diagnostics: found.with(Diagnostic::error(
                Code::ClaudeAgent,
                at,
                Message::new("agent ")
                    .then(MessagePart::printable(agent.name()))
                    .then(": the native field ")
                    .then(Stripped::from_untrusted(&key))
                    .then(
                        " holds a null, a number that is not whole or a number beyond 64 bits, \
                         which alz does not write",
                    ),
            )),
        },
    }
}

/// The `tools` field: the native names without duplicates, and a warning for each scope Claude
/// cannot keep.
fn native_tools(agent: &MergedAgent, at: Option<&Where>) -> (Tools, Diagnostics) {
    let name = MessagePart::printable(agent.name());
    match agent.tools() {
        None => (Tools::Inherit, Diagnostics::new()),
        Some(grants) if grants.is_empty() => (
            Tools::Empty,
            Diagnostics::new().with(Diagnostic::error(
                Code::ClaudeAgent,
                at.cloned(),
                Message::new("agent ")
                    .then(name)
                    .then(" has an empty tools list, which Claude reads as \"all tools\""),
            )),
        ),
        Some(grants) => {
            let warnings = grants
                .iter()
                .filter(|grant| grant.scope().is_some())
                .map(|grant| scope_warning(agent, grant, at))
                .collect();
            let names = grants.iter().map(|grant| tools::tool(grant.tool())).fold(
                Vec::<String>::new(),
                |names, native| {
                    if names.contains(&native) {
                        names
                    } else {
                        names.into_iter().chain([native]).collect()
                    }
                },
            );
            (Tools::Listed(names.join(", ")), warnings)
        }
    }
}

fn scope_warning(agent: &MergedAgent, grant: &ToolGrant, at: Option<&Where>) -> Diagnostic {
    Diagnostic::warning(
        Code::ToolScope,
        at.cloned(),
        Message::new("agent ")
            .then(MessagePart::printable(agent.name()))
            .then(": Claude cannot scope a tool per agent, so '")
            .then(Stripped::from_untrusted(&grant.to_string()))
            .then("' becomes '")
            .then(Stripped::from_untrusted(&tools::tool(grant.tool())))
            .then("'; put the scope in permissions.yaml"),
    )
}

/// The Claude model of an agent through the routing (§7.8): an explicit alias, else the role's
/// alias. `None` means the agent uses the session's model.
fn native_model(
    agent: &MergedAgent,
    models: &MergedModels,
    at: Option<&Where>,
) -> (Option<String>, Diagnostics) {
    let name = || MessagePart::printable(agent.name());
    let alias = match agent
        .scalars()
        .model()
        .filter(|id| id.as_str() != "inherit")
    {
        Some(id) => ModelAlias::try_from(id.as_str())
            .ok()
            .filter(|alias| models.aliases().contains_key(alias))
            .map(Some)
            .ok_or_else(|| {
                Diagnostic::error(
                    Code::Models,
                    at.cloned(),
                    Message::new("agent ")
                        .then(name())
                        .then(": model '")
                        .then(Stripped::from_untrusted(id.as_str()))
                        .then("' is not an alias in models.yaml; concrete ids live only there"),
                )
            }),
        None => Ok(agent
            .scalars()
            .role()
            .and_then(|role| models.alias_of(role))
            .cloned()),
    };
    match alias {
        Err(problem) => (None, Diagnostics::new().with(problem)),
        Ok(None) => (None, Diagnostics::new()),
        Ok(Some(alias)) => match models.model(&alias, crate::target::claude()) {
            None => (
                None,
                Diagnostics::new().with(Diagnostic::warning(
                    Code::Models,
                    at.cloned(),
                    Message::new("alias '")
                        .then(Stripped::from_untrusted(alias.as_str()))
                        .then("' has no claude model; agent ")
                        .then(name())
                        .then(" inherits"),
                )),
            ),
            Some(id) if tools::is_model_value(id.as_str()) => {
                (Some(id.as_str().to_owned()), Diagnostics::new())
            }
            Some(id) => (
                Some(id.as_str().to_owned()),
                Diagnostics::new().with(Diagnostic::error(
                    Code::ClaudeAgent,
                    at.cloned(),
                    Message::new("agent ")
                        .then(name())
                        .then(": '")
                        .then(Stripped::from_untrusted(id.as_str()))
                        .then("' is not a Claude model value"),
                )),
            ),
        },
    }
}

/// The native fields of Claude as emitted entries, or the first key whose value alz cannot write.
fn native_fields(agent: &MergedAgent) -> Result<Vec<(String, EmitValue)>, String> {
    agent.native().of(crate::target::claude()).map_or_else(
        || Ok(Vec::new()),
        |map| {
            map.iter()
                .map(|(key, value)| {
                    emitted(value)
                        .map(|value| (key.to_owned(), value))
                        .ok_or_else(|| key.to_owned())
                })
                .collect()
        },
    )
}

/// A native value as YAML alz writes: no null, no float and no integer beyond 64 bits.
fn emitted(value: &NativeValue) -> Option<EmitValue> {
    match value {
        NativeValue::Null | NativeValue::Float(_) => None,
        NativeValue::Bool(flag) => Some(EmitValue::Bool(*flag)),
        NativeValue::Int(number) => i64::try_from(*number).ok().map(EmitValue::Int),
        NativeValue::Text(value) => Some(text(value)),
        NativeValue::List(items) => items
            .iter()
            .map(emitted)
            .collect::<Option<Vec<_>>>()
            .map(EmitValue::Seq),
        NativeValue::Map(map) => map
            .iter()
            .map(|(key, value)| emitted(value).map(|value| (key.to_owned(), value)))
            .collect::<Option<Vec<_>>>()
            .and_then(|entries| EmitMap::from_entries(entries).ok())
            .map(EmitValue::Map),
    }
}
