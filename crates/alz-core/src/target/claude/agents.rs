//! `.claude/agents/<name>.md` (spec §8.1, target-facts §1): an agent of the model, its neutral
//! fields mapped to the native frontmatter, then its native fields in source order.

use alz_adapter::{
    Agent, AgentTool, Diagnostic, Isolation, JsonValue, Level, Model, ModelProblem, Source,
};

use super::frontmatter::{file, header_sources, text};
use super::{Made, found, place, tools};

/// The folder of the agent files.
const AGENTS_DIR: &str = ".claude/agents";

/// The agents as files, with the errors and warnings of the mapping.
pub(super) fn render(model: &Model) -> Made {
    model
        .agents
        .iter()
        .map(one)
        .fold(Made::default(), Made::then)
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

fn one(agent: &Agent) -> Made {
    let at = place(&agent.sources);
    let (tools_field, tool_found) = native_tools(agent, at);
    if matches!(tools_field, Tools::Empty) {
        return Made {
            files: Vec::new(),
            diagnostics: tool_found,
        };
    }
    let (model, model_found) = native_model(agent, at);
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
                JsonValue::Array(agent.skills.iter().map(|skill| text(skill)).collect()),
            )
        }),
        matches!(agent.isolation, Isolation::Worktree).then(|| ("isolation", text("worktree"))),
        tools::memory(agent.memory).map(|memory| ("memory", text(memory))),
    ]
    .into_iter()
    .flatten()
    .map(|(key, value)| (key.to_owned(), value));
    let found_so_far: Vec<Diagnostic> = tool_found.into_iter().chain(model_found).collect();
    match native_fields(agent) {
        Ok(native) => file(
            &format!("{AGENTS_DIR}/{}.md", agent.name),
            header_sources(&agent.sources),
            fixed.chain(native).collect(),
            &agent.body,
        )
        .after(found_so_far),
        Err(key) => Made::problem(found(
            Level::Error,
            "claude-agent",
            at,
            format!(
                "agent {}: the native field {key} holds a null, a number that is not whole or a number beyond 64 bits, which alz does not write",
                agent.name
            ),
        ))
        .after(found_so_far),
    }
}

/// The `tools` field: the native names without duplicates, and a warning for each scope Claude
/// cannot keep.
fn native_tools(agent: &Agent, at: Option<&Source>) -> (Tools, Vec<Diagnostic>) {
    match agent.tools.as_deref() {
        None => (Tools::Inherit, Vec::new()),
        Some([]) => (
            Tools::Empty,
            vec![found(
                Level::Error,
                "claude-agent",
                at,
                format!(
                    "agent {} has an empty tools list, which Claude reads as \"all tools\"",
                    agent.name
                ),
            )],
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

fn scope_warning(agent: &Agent, grant: &AgentTool, at: Option<&Source>) -> Diagnostic {
    let written = grant.scope.as_ref().map_or_else(
        || grant.name.clone(),
        |scope| format!("{}:{scope}", grant.name),
    );
    found(
        Level::Warning,
        "tool-scope",
        at,
        format!(
            "agent {}: Claude cannot scope a tool per agent, so '{written}' becomes '{}'; put the scope in permissions.yaml",
            agent.name,
            tools::tool(&grant.name)
        ),
    )
}

/// The Claude model of an agent (§7.8), which the core resolved through the routing: `None` means
/// the agent uses the session's model.
fn native_model(agent: &Agent, at: Option<&Source>) -> (Option<String>, Vec<Diagnostic>) {
    let name = &agent.name;
    match (&agent.model, &agent.model_problem) {
        (_, Some(ModelProblem::NotAnAlias(id))) => (
            None,
            vec![found(
                Level::Error,
                "models",
                at,
                format!(
                    "agent {name}: model '{id}' is not an alias in models.yaml; concrete ids live only there"
                ),
            )],
        ),
        (_, Some(ModelProblem::NoModelForTarget(alias))) => (
            None,
            vec![found(
                Level::Warning,
                "models",
                at,
                format!("alias '{alias}' has no claude model; agent {name} inherits"),
            )],
        ),
        (None, None) => (None, Vec::new()),
        (Some(id), None) if tools::is_model_value(id) => (Some(id.clone()), Vec::new()),
        (Some(id), None) => (
            Some(id.clone()),
            vec![found(
                Level::Error,
                "claude-agent",
                at,
                format!("agent {name}: '{id}' is not a Claude model value"),
            )],
        ),
    }
}

/// The native fields of Claude as frontmatter entries, or the first key whose value alz cannot
/// write.
fn native_fields(agent: &Agent) -> Result<Vec<(String, JsonValue)>, String> {
    agent
        .native
        .iter()
        .map(|(key, value)| {
            writable(value)
                .then(|| (key.clone(), value.clone()))
                .ok_or_else(|| key.clone())
        })
        .collect()
}

/// Whether a native value is YAML alz writes: no null, no float and no integer beyond 64 bits.
fn writable(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null | JsonValue::Float(_) => false,
        JsonValue::Bool(_) | JsonValue::Text(_) => true,
        JsonValue::Int(number) => i64::try_from(*number).is_ok(),
        JsonValue::Array(items) => items.iter().all(writable),
        JsonValue::Object(map) => map.entries().all(|(_, value)| writable(value)),
    }
}
