//! `.claude/agents/<name>.md` (spec §8.1, target-facts §1): an agent of the model, its neutral
//! fields mapped to the native frontmatter, then its native fields in source order.

use alz_adapter::{Agent, AgentTool, Isolation, JsonValue, Model, ModelProblem};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::render::yaml::{EmitMap, EmitValue};
use crate::untrusted_text::Stripped;

use super::frontmatter::{file, place, sources, text};
use super::{Output, tools};

/// The folder of the agent files.
const AGENTS_DIR: &str = ".claude/agents";

/// The agents as files, with the errors and warnings of the mapping.
pub(super) fn render(model: &Model) -> Output {
    model
        .agents
        .iter()
        .map(one)
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

fn one(agent: &Agent) -> Output {
    let at = place(&agent.sources);
    let (tools_field, tool_found) = native_tools(agent, at.as_ref());
    if matches!(tools_field, Tools::Empty) {
        return Output {
            files: Vec::new(),
            diagnostics: tool_found,
        };
    }
    let (model, model_found) = native_model(agent, at.as_ref());
    let fixed = [
        Some(("name", text(&agent.name))),
        Some(("description", text(&agent.description))),
        match tools_field {
            Tools::Listed(names) => Some(("tools", text(&names))),
            Tools::Inherit | Tools::Empty => None,
        },
        model.map(|value| ("model", text(&value))),
        agent
            .permission_mode
            .map(|mode| ("permissionMode", text(tools::permission_mode(mode)))),
        (!agent.skills.is_empty()).then(|| {
            (
                "skills",
                EmitValue::Seq(agent.skills.iter().map(|skill| text(skill)).collect()),
            )
        }),
        matches!(agent.isolation, Isolation::Worktree).then(|| ("isolation", text("worktree"))),
        tools::memory(agent.memory).map(|memory| ("memory", text(memory))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value));
    let found = tool_found.merge(model_found);
    match native_fields(agent) {
        Ok(native) => file(
            &format!("{AGENTS_DIR}/{}.md", agent.name),
            sources(&agent.sources),
            fixed.chain(native).collect(),
            &agent.body,
            found,
        ),
        Err(key) => Output {
            files: Vec::new(),
            diagnostics: found.with(Diagnostic::error(
                Code::ClaudeAgent,
                at,
                Message::new("agent ")
                    .then(Stripped::from_untrusted(&agent.name))
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
fn native_tools(agent: &Agent, at: Option<&Where>) -> (Tools, Diagnostics) {
    match agent.tools.as_deref() {
        None => (Tools::Inherit, Diagnostics::new()),
        Some([]) => (
            Tools::Empty,
            Diagnostics::new().with(Diagnostic::error(
                Code::ClaudeAgent,
                at.cloned(),
                Message::new("agent ")
                    .then(Stripped::from_untrusted(&agent.name))
                    .then(" has an empty tools list, which Claude reads as \"all tools\""),
            )),
        ),
        Some(grants) => {
            let warnings = grants
                .iter()
                .filter(|grant| grant.scope.is_some())
                .map(|grant| scope_warning(agent, grant, at))
                .collect();
            let names = grants.iter().map(|grant| tools::tool(&grant.name)).fold(
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

fn scope_warning(agent: &Agent, grant: &AgentTool, at: Option<&Where>) -> Diagnostic {
    let written = grant.scope.as_ref().map_or_else(
        || grant.name.clone(),
        |scope| format!("{}:{scope}", grant.name),
    );
    Diagnostic::warning(
        Code::ToolScope,
        at.cloned(),
        Message::new("agent ")
            .then(Stripped::from_untrusted(&agent.name))
            .then(": Claude cannot scope a tool per agent, so '")
            .then(Stripped::from_untrusted(&written))
            .then("' becomes '")
            .then(Stripped::from_untrusted(&tools::tool(&grant.name)))
            .then("'; put the scope in permissions.yaml"),
    )
}

/// The Claude model of an agent (§7.8), which the core resolved through the routing: `None` means
/// the agent uses the session's model.
fn native_model(agent: &Agent, at: Option<&Where>) -> (Option<String>, Diagnostics) {
    let name = || Stripped::from_untrusted(&agent.name);
    match (&agent.model, &agent.model_problem) {
        (_, Some(ModelProblem::NotAnAlias(id))) => (
            None,
            Diagnostics::new().with(Diagnostic::error(
                Code::Models,
                at.cloned(),
                Message::new("agent ")
                    .then(name())
                    .then(": model '")
                    .then(Stripped::from_untrusted(id))
                    .then("' is not an alias in models.yaml; concrete ids live only there"),
            )),
        ),
        (_, Some(ModelProblem::NoModelForTarget(alias))) => (
            None,
            Diagnostics::new().with(Diagnostic::warning(
                Code::Models,
                at.cloned(),
                Message::new("alias '")
                    .then(Stripped::from_untrusted(alias))
                    .then("' has no claude model; agent ")
                    .then(name())
                    .then(" inherits"),
            )),
        ),
        (None, None) => (None, Diagnostics::new()),
        (Some(id), None) if tools::is_model_value(id) => (Some(id.clone()), Diagnostics::new()),
        (Some(id), None) => (
            Some(id.clone()),
            Diagnostics::new().with(Diagnostic::error(
                Code::ClaudeAgent,
                at.cloned(),
                Message::new("agent ")
                    .then(name())
                    .then(": '")
                    .then(Stripped::from_untrusted(id))
                    .then("' is not a Claude model value"),
            )),
        ),
    }
}

/// The native fields of Claude as emitted entries, or the first key whose value alz cannot write.
fn native_fields(agent: &Agent) -> Result<Vec<(String, EmitValue)>, String> {
    agent
        .native
        .iter()
        .map(|(key, value)| {
            emitted(value)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| key.clone())
        })
        .collect()
}

/// A native value as YAML alz writes: no null, no float and no integer beyond 64 bits.
fn emitted(value: &JsonValue) -> Option<EmitValue> {
    match value {
        JsonValue::Null | JsonValue::Float(_) => None,
        JsonValue::Bool(flag) => Some(EmitValue::Bool(*flag)),
        JsonValue::Int(number) => i64::try_from(*number).ok().map(EmitValue::Int),
        JsonValue::Text(value) => Some(text(value)),
        JsonValue::Array(items) => items
            .iter()
            .map(emitted)
            .collect::<Option<Vec<_>>>()
            .map(EmitValue::Seq),
        JsonValue::Object(map) => map
            .entries()
            .map(|(key, value)| emitted(value).map(|value| (key.to_owned(), value)))
            .collect::<Option<Vec<_>>>()
            .and_then(|entries| EmitMap::from_entries(entries).ok())
            .map(EmitValue::Map),
    }
}
