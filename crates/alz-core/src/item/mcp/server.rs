//! One server of `mcp.yaml` (spec §7.7): how it is started or reached, as a sum type, and the
//! reader that turns the mapping of a file into one.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::schema::{
    Checked, SchemaError, SchemaErrors, require, text_list, unknown_fields, zip, zip3,
};
use crate::secret;
use crate::yaml::fields::{KeyPath, Section};
use crate::yaml::{Node, Value};

use super::super::protected_variable::env_from_names;
use super::super::{EnvVarName, schema_diagnostics};
use super::server_id::{McpRefusal, McpServerId};

/// The fields of a server.
const FIELDS: [&str; 5] = ["transport", "url", "command", "args", "env_from"];

/// The fields that would hold a value, which a server never stores (§7.7).
const VALUE_FIELDS: [&str; 2] = ["env", "headers"];

/// The `transport` of a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportKind {
    Stdio,
    Http,
    Sse,
}

impl TryFrom<&str> for TransportKind {
    type Error = McpRefusal;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        match text {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            "sse" => Ok(Self::Sse),
            _ => Err(McpRefusal::Transport),
        }
    }
}

/// How a server is reached. `env_from` exists only on a server that is started, by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    /// A process that alz's target starts and talks to over its standard streams.
    Stdio {
        /// The program to run.
        command: String,
        /// Its arguments, in order.
        args: Vec<String>,
        /// The names of the environment variables passed through to it; never their values.
        env_from: Vec<EnvVarName>,
    },
    /// A server reached by streamable HTTP.
    Http {
        /// Where it is.
        url: String,
    },
    /// A server reached by server-sent events.
    Sse {
        /// Where it is.
        url: String,
    },
}

/// One MCP server definition. Its id is the key it is stored under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    transport: Transport,
}

impl McpServer {
    /// How the server is reached.
    #[must_use]
    pub const fn transport(&self) -> &Transport {
        &self.transport
    }
}

/// Why a server gives no item.
#[derive(Debug)]
pub(super) enum Rejected {
    /// A problem of the definition.
    Reported(Diagnostics),
    /// A protected or malformed variable in `env_from`: the `mcp.enable` check does not add a
    /// second error for the server (§7.9).
    ProtectedVariable(Diagnostics),
}

fn reported(diagnostic: Diagnostic) -> Rejected {
    Rejected::Reported(Diagnostics::new().with(diagnostic))
}

/// The start of a message about server `id`: `server <id>: `.
fn server_head(id: &McpServerId) -> Message {
    Message::new("server ")
        .then(MessagePart::printable(id))
        .then(": ")
}

/// The error of server `id`, at `at`: `server <id>: <what>`.
fn server_error(id: &McpServerId, at: Where, what: &'static str) -> Diagnostic {
    Diagnostic::error(Code::Mcp, Some(at), server_head(id).then(what))
}

/// The text values of a definition: its text fields and the text items of its lists.
fn text_values<'a>(section: &Section<'a>) -> Vec<&'a str> {
    FIELDS
        .iter()
        .filter_map(|field| section.node(field))
        .flat_map(|node| match node.value() {
            Value::Text(text) => vec![text.as_str()],
            Value::Seq(items) => items
                .iter()
                .filter_map(|item| match item.value() {
                    Value::Text(text) => Some(text.as_str()),
                    Value::Null
                    | Value::Bool(_)
                    | Value::Int(_)
                    | Value::Float(_)
                    | Value::Seq(_)
                    | Value::Map(_) => None,
                })
                .collect(),
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Map(_) => {
                Vec::new()
            }
        })
        .collect()
}

fn optional_text<'a>(section: &Section<'a>, field: &'static str) -> Checked<Option<&'a str>> {
    section.text(field).map_err(SchemaErrors::from)
}

/// Reads the definition at `node` of the server `id`, where `at` is the file. The checks run in
/// the order of the spec and the first that finds a problem ends the server: its shape, the
/// fields that would hold values, unknown fields, a value that looks like a token, then the fields
/// themselves, then `env_from`.
pub(super) fn read_server(
    id: &McpServerId,
    node: &Node,
    at: &Where,
) -> Result<McpServer, Rejected> {
    let here = at.clone().at(node.at());
    let section = Section::new(node, KeyPath::root()).map_err(|_not_a_mapping| {
        reported(server_error(
            id,
            here.clone(),
            "the definition must be a mapping",
        ))
    })?;
    VALUE_FIELDS
        .iter()
        .find_map(|field| section.key_position(field))
        .map_or(Ok(()), |position| {
            Err(reported(server_error(
                id,
                at.clone().at(position),
                "values are never stored here; list variable names in 'env_from'",
            )))
        })?;
    unknown_fields(&section, &FIELDS)
        .map_err(|errors| Rejected::Reported(schema_diagnostics(Code::Mcp, at, &errors)))?;
    let kinds: Vec<secret::SecretKind> = text_values(&section)
        .into_iter()
        .flat_map(secret::scan)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if !kinds.is_empty() {
        return Err(Rejected::Reported(
            kinds
                .into_iter()
                .map(|kind| {
                    Diagnostic::error(
                        Code::LeakSecret,
                        Some(here.clone()),
                        server_head(id)
                            .then("a value looks like a ")
                            .then(kind.name()),
                    )
                })
                .collect(),
        ));
    }
    let ((kind, command, url), (args, env_from)) = zip(
        zip3(
            require::<TransportKind>(&section, "transport"),
            optional_text(&section, "command"),
            optional_text(&section, "url"),
        ),
        zip(
            text_list::<String>(&section, "args"),
            text_list::<String>(&section, "env_from"),
        ),
    )
    .map_err(|errors| Rejected::Reported(schema_diagnostics(Code::Mcp, at, &errors)))?;
    let needed = |field: &'static str, value: Option<&str>| {
        value.map(str::to_owned).ok_or_else(|| {
            Rejected::Reported(schema_diagnostics(
                Code::Mcp,
                at,
                &SchemaError::missing(section.field_path(field), section.at()).into(),
            ))
        })
    };
    match kind {
        TransportKind::Stdio => {
            let command = needed("command", command)?;
            let env_from = env_names(&env_from, &section, at)?;
            Ok(McpServer {
                transport: Transport::Stdio {
                    command,
                    args,
                    env_from,
                },
            })
        }
        TransportKind::Http | TransportKind::Sse => {
            let url = needed("url", url)?;
            if env_from.is_empty() {
                Ok(McpServer {
                    transport: match kind {
                        TransportKind::Http => Transport::Http { url },
                        TransportKind::Stdio | TransportKind::Sse => Transport::Sse { url },
                    },
                })
            } else {
                Err(reported(server_error(
                    id,
                    at.clone()
                        .at(section.key_position("env_from").unwrap_or(section.at())),
                    "'env_from' only applies to stdio servers",
                )))
            }
        }
    }
}

/// The names of `env_from` as variable names, or the problem of the names that are protected or
/// not plain names.
fn env_names(
    names: &[String],
    section: &Section<'_>,
    at: &Where,
) -> Result<Vec<EnvVarName>, Rejected> {
    match names
        .iter()
        .map(|name| EnvVarName::try_from(name.as_str()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(parsed) if parsed.iter().all(|name| name.protected().is_none()) => Ok(parsed),
        Ok(_) | Err(_) => Err(Rejected::ProtectedVariable(env_from_names(
            names.iter().map(String::as_str),
            &at.clone()
                .at(section.key_position("env_from").unwrap_or(section.at())),
        ))),
    }
}
