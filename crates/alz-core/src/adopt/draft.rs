//! The plan while it is made: what each native file adds to the repo layer, and every problem
//! found on the way. The conversions of the native files are independent steps that add to one
//! draft, so the draft is mutated in place; it never leaves this module's callers and the plan
//! that comes out of it is an immutable value.

use std::collections::BTreeMap;

use crate::diagnostic::{Code, Diagnostic, Message, Where};
use crate::item::AgentName;
use crate::layer::{LayerContents, LayerName};
use crate::lint::parse_document;
use crate::merge::Merged;
use crate::path::SafeRelPath;
use crate::render::yaml::{EmitValue, emit};
use crate::untrusted_text::{QuotedKey, Stripped};

use super::NativeFile;
use super::values::{from_node, merge};

/// What the planner knows about the repository besides the native files.
#[derive(Debug, Clone, Copy)]
pub(super) struct Context<'a> {
    /// The repo layer's folder from the repository root, such as `.layer`.
    pub layer_dir: &'a SafeRelPath,
    /// The repo layer's name.
    pub layer_name: &'a LayerName,
    /// The files the repo layer has.
    pub existing: &'a LayerContents,
    /// The stack as it merges before adoption.
    pub merged: &'a Merged,
    /// The bytes of `AGENTS.md` as it is on disk, when there is one.
    pub agents_md: Option<&'a [u8]>,
}

impl Context<'_> {
    /// Whether the stack before adoption defines the agent called `name`.
    pub(super) fn defines_agent(&self, name: &str) -> bool {
        AgentName::try_from(name).is_ok_and(|name| self.merged.agents().contains_key(&name))
    }
}

/// One file of the repo layer that the plan writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Planned {
    /// Text as it is.
    Text(Vec<u8>),
    /// A YAML document, which later conversions may add to.
    Yaml(EmitValue),
}

/// A native file moved byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Moved {
    pub from: SafeRelPath,
    pub to: SafeRelPath,
    pub file: NativeFile,
}

/// What one native file became, for the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Reported {
    pub from: String,
    pub to: Vec<String>,
}

/// The plan under construction.
#[derive(Debug)]
pub(super) struct Draft<'a> {
    pub ctx: Context<'a>,
    pub files: BTreeMap<SafeRelPath, (String, Planned)>,
    pub moves: Vec<Moved>,
    pub removes: Vec<String>,
    pub add_dirs: Vec<String>,
    pub mcp_enable: Vec<String>,
    pub report: Vec<Reported>,
    pub notes: Vec<Diagnostic>,
    pub problems: Vec<Diagnostic>,
}

impl<'a> Draft<'a> {
    pub(super) const fn new(ctx: Context<'a>) -> Self {
        Self {
            ctx,
            files: BTreeMap::new(),
            moves: Vec::new(),
            removes: Vec::new(),
            add_dirs: Vec::new(),
            mcp_enable: Vec::new(),
            report: Vec::new(),
            notes: Vec::new(),
            problems: Vec::new(),
        }
    }

    /// A problem with the native file `file`. The message may hold the file's text only as
    /// stripped or quoted parts, never raw.
    pub(super) fn problem(&mut self, file: &str, message: Message) {
        self.problems.push(Diagnostic::error(
            Code::Adopt,
            Some(Where::repository_file(Stripped::from_untrusted(file))),
            message,
        ));
    }

    /// A note about how the native file `file` was adopted.
    pub(super) fn note(&mut self, file: &str, message: Message) {
        self.notes.push(Diagnostic::warning(
            Code::Adopt,
            Some(Where::repository_file(Stripped::from_untrusted(file))),
            message,
        ));
    }

    /// A problem that a value of the file cannot be written to a layer file.
    pub(super) fn unwritable(&mut self, file: &str, key: &str, why: &str) {
        self.problem(
            file,
            Message::new("'")
                .then(Stripped::from_untrusted(key))
                .then("' holds ")
                .then(Stripped::from_untrusted(why))
                .then(", which a layer file cannot hold; move it by hand"),
        );
    }

    /// The layer path `path`, or a problem when it is not a safe path.
    fn layer_path(&mut self, file: &str, path: &str) -> Option<SafeRelPath> {
        let made = SafeRelPath::try_from(path).ok();
        if made.is_none() {
            self.problem(
                file,
                Message::new("the layer path ")
                    .then(QuotedKey::from_untrusted(path))
                    .then(" is not a safe relative path"),
            );
        }
        made
    }

    fn exists(&self, path: &SafeRelPath) -> bool {
        self.files.contains_key(path)
            || self.ctx.existing.get(path).is_some()
            || self.moves.iter().any(|moved| &moved.to == path)
    }

    fn already_there(&mut self, file: &str, path: &SafeRelPath) {
        self.problem(
            file,
            Message::new(Stripped::from_untrusted(&format!(
                "{}/{path}",
                self.ctx.layer_dir
            )))
            .then(" already exists"),
        );
    }

    /// Queues a text file of the repo layer, which must not exist yet.
    pub(super) fn write_text(&mut self, file: &str, path: &str, text: String) {
        let Some(path) = self.layer_path(file, path) else {
            return;
        };
        if self.exists(&path) {
            self.already_there(file, &path);
        } else {
            self.files
                .insert(path, (file.to_owned(), Planned::Text(text.into_bytes())));
        }
    }

    /// Queues YAML for a layer file, merged into what the layer holds or the plan queued already
    /// (mappings key by key, lists appended without duplicates).
    pub(super) fn merge_yaml(&mut self, file: &str, path: &str, value: EmitValue) {
        let Some(path) = self.layer_path(file, path) else {
            return;
        };
        let held = match self.files.get(&path) {
            Some((_, Planned::Yaml(queued))) => Ok(Some(queued.clone())),
            Some((_, Planned::Text(_))) => {
                self.problem(
                    file,
                    Message::new(Stripped::from_untrusted(path.as_str())).then(" is written twice"),
                );
                return;
            }
            None => self.ctx.existing.get(&path).map_or(Ok(None), read_yaml),
        };
        let merged = match held {
            Ok(None) => Some(value),
            Ok(Some(held)) => merge(held, value),
            Err(reason) => {
                self.problem(
                    file,
                    Message::new("cannot merge into ")
                        .then(Stripped::from_untrusted(&format!(
                            "{}/{path}",
                            self.ctx.layer_dir
                        )))
                        .then(": it holds ")
                        .then(Stripped::from_untrusted(reason)),
                );
                return;
            }
        };
        match merged {
            Some(merged) => {
                self.files
                    .insert(path, (file.to_owned(), Planned::Yaml(merged)));
            }
            None => self.problem(
                file,
                Message::new(Stripped::from_untrusted(&format!(
                    "{}/{path}",
                    self.ctx.layer_dir
                )))
                .then(" has a different shape; merge it by hand"),
            ),
        }
    }

    /// Queues the move of the native file `from` to the layer path `to`.
    pub(super) fn move_file(&mut self, from: &SafeRelPath, file: &NativeFile, to: &str) {
        let Some(to) = self.layer_path(from.as_str(), to) else {
            return;
        };
        if self.exists(&to) {
            self.already_there(from.as_str(), &to);
            return;
        }
        self.report.push(Reported {
            from: from.as_str().to_owned(),
            to: vec![format!("{}/{to}", self.ctx.layer_dir)],
        });
        self.moves.push(Moved {
            from: from.clone(),
            to,
            file: file.clone(),
        });
    }

    /// Records that the native file `from` became the layer files `to` (paths from the layer,
    /// or `layer.yaml`) and is removed once they are written.
    pub(super) fn converted(&mut self, from: &str, to: &[String]) {
        self.removes.push(from.to_owned());
        self.report.push(Reported {
            from: from.to_owned(),
            to: to
                .iter()
                .map(|path| {
                    if path == "layer.yaml" {
                        path.clone()
                    } else {
                        format!("{}/{path}", self.ctx.layer_dir)
                    }
                })
                .collect(),
        });
    }

    /// The bytes the plan writes for each layer file.
    pub(super) fn layer_files(&self) -> BTreeMap<SafeRelPath, Vec<u8>> {
        self.files
            .iter()
            .map(|(path, (_, planned))| {
                (
                    path.clone(),
                    match planned {
                        Planned::Text(bytes) => bytes.clone(),
                        Planned::Yaml(value) => emit(value).into_bytes(),
                    },
                )
            })
            .collect()
    }
}

/// The YAML of a layer file as a value the emitter writes: `Ok(None)` for a file without a value.
fn read_yaml(bytes: &[u8]) -> Result<Option<EmitValue>, &'static str> {
    let node = parse_document(bytes).map_err(|_unreadable| "YAML that alz refuses")?;
    if matches!(node.value(), crate::yaml::Value::Null) {
        Ok(None)
    } else {
        from_node(&node)
            .map(Some)
            .map_err(super::values::Unwritable::describe)
    }
}
