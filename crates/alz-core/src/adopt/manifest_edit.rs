//! The edit of the repository's `layer.yaml` that adoption makes (spec §4.3 "Adoption, in
//! detail"): each adopted directory is appended to `workspace.add_dirs` and each adopted server id
//! to `mcp.enable`. The file is edited as text and never written again from a parsed tree, so
//! comments and every other byte stay. A form this edit does not handle is refused, and the
//! edited text must read back as the same manifest with the values added.

use crate::lint::parse_document;
use crate::render::yaml::{EmitValue, emit};
use crate::stack::Manifest;

/// The manifest as edited, with the text that holds it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Edited {
    pub text: String,
    pub manifest: Manifest,
}

/// Why `layer.yaml` is not edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refused {
    /// The list or its section has a form this edit does not handle.
    Form,
    /// The edited text does not read as the manifest with the values added.
    ReadsBack,
}

/// The step of YAML indentation alz writes.
const STEP: &str = "  ";

/// `text` with the directories and server ids that `manifest` lacks added, or `None` when it holds
/// them all.
///
/// # Errors
///
/// [`Refused`] for a form this edit does not handle and for text that would not read back.
pub(super) fn edit(
    text: &str,
    manifest: &Manifest,
    add_dirs: &[String],
    mcp_enable: &[String],
) -> Result<Option<Edited>, Refused> {
    let held_dirs: Vec<String> = manifest
        .workspace()
        .add_dirs()
        .iter()
        .map(|dir| dir.as_str().to_owned())
        .collect();
    let held_ids: Vec<String> = manifest
        .mcp_enable()
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    let new_dirs = fresh(add_dirs, &held_dirs);
    let new_ids = fresh(mcp_enable, &held_ids);
    if new_dirs.is_empty() && new_ids.is_empty() {
        return Ok(None);
    }
    if has_several_documents(text) {
        return Err(Refused::Form);
    }
    let edited = add_list(text, "workspace", "add_dirs", &new_dirs)
        .and_then(|text| add_list(&text, "mcp", "enable", &new_ids))?;
    let read = parse_document(edited.as_bytes())
        .ok()
        .and_then(|node| Manifest::from_yaml(&node).ok())
        .ok_or(Refused::ReadsBack)?;
    let expected_dirs: Vec<String> = held_dirs.into_iter().chain(new_dirs).collect();
    let expected_ids: Vec<String> = held_ids.into_iter().chain(new_ids).collect();
    let unchanged = read.stack() == manifest.stack()
        && read.targets() == manifest.targets()
        && read.protected_paths() == manifest.protected_paths()
        && read.budgets() == manifest.budgets()
        && read.options() == manifest.options()
        && read.workspace().hub() == manifest.workspace().hub()
        && read
            .workspace()
            .add_dirs()
            .iter()
            .map(|dir| dir.as_str().to_owned())
            .eq(expected_dirs)
        && read
            .mcp_enable()
            .iter()
            .map(|id| id.as_str().to_owned())
            .eq(expected_ids);
    if unchanged {
        Ok(Some(Edited {
            text: edited,
            manifest: read,
        }))
    } else {
        Err(Refused::ReadsBack)
    }
}

/// The values of `wanted` that `held` lacks, each once, in order.
fn fresh(wanted: &[String], held: &[String]) -> Vec<String> {
    wanted.iter().fold(Vec::new(), |mut fresh, value| {
        if !held.contains(value) && !fresh.contains(value) {
            fresh.push(value.clone());
        }
        fresh
    })
}

/// Whether a line starts another document: `---` after the first line, or `...`.
fn has_several_documents(text: &str) -> bool {
    text.lines().enumerate().any(|(at, line)| {
        let marker = |mark: &str| {
            line.strip_prefix(mark)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
        };
        marker("...") || (at > 0 && marker("---"))
    })
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// The value as one line of YAML, plain when YAML reads it back as the same text, else quoted as
/// §10.2 writes text. In a flow list a value that holds a flow indicator is double-quoted.
fn scalar(value: &str, flow: bool) -> Option<String> {
    let written = emit(&EmitValue::Text(value.to_owned()));
    let line = written.strip_suffix('\n')?;
    if line.contains('\n') {
        return None;
    }
    let quoted = line.starts_with(['"', '\'']);
    if flow && !quoted && line.contains([',', '[', ']', '{', '}']) {
        Some(format!(
            "\"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        ))
    } else {
        Some(line.to_owned())
    }
}

/// `text` with `values` appended to the list `key` of the section `section`.
fn add_list(text: &str, section: &str, key: &str, values: &[String]) -> Result<String, Refused> {
    if values.is_empty() {
        return Ok(text.to_owned());
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let header = format!("{section}:");
    let Some(at) = lines.iter().position(|line| line.starts_with(&header)) else {
        return append_section(text, section, key, values);
    };
    let rest = lines
        .get(at)
        .and_then(|line| line.strip_prefix(header.as_str()))
        .ok_or(Refused::Form)?;
    if !is_blank_or_comment(rest) {
        return Err(Refused::Form);
    }
    // The section runs to the next line that starts in the first column and is not a comment.
    let end = lines
        .iter()
        .enumerate()
        .skip(at + 1)
        .find(|(_, line)| !line.trim().is_empty() && !line.starts_with([' ', '\t', '#']))
        .map_or(lines.len(), |(end, _)| end);
    let body = || lines.iter().enumerate().take(end).skip(at + 1);
    let last = body()
        .filter(|(_, line)| !is_blank_or_comment(line))
        .map(|(index, _)| index)
        .next_back()
        .unwrap_or(at);
    let child = body()
        .find(|(_, line)| !is_blank_or_comment(line))
        .map_or(STEP.len(), |(_, line)| indent_of(line));
    let key_line = body().find(|(_, line)| {
        indent_of(line) == child
            && line
                .trim_start()
                .strip_prefix(key)
                .is_some_and(|r| r.starts_with(':'))
    });
    match key_line {
        None => Ok(insert_after(
            &lines,
            last,
            &block(
                &format!("{}{key}:\n", " ".repeat(child)),
                child + STEP.len(),
                values,
            )?,
        )),
        Some((k, line)) => {
            let value = line
                .trim_start()
                .strip_prefix(key)
                .and_then(|after| after.strip_prefix(':'))
                .ok_or(Refused::Form)?;
            if is_blank_or_comment(value) {
                block_list(&lines, k, end, child, values)
            } else {
                flow_list(&lines, k, value, values)
            }
        }
    }
}

/// `head` followed by one `- <value>` line per value at `indent`.
fn block(head: &str, indent: usize, values: &[String]) -> Result<String, Refused> {
    values
        .iter()
        .map(|value| scalar(value, false).ok_or(Refused::Form))
        .try_fold(head.to_owned(), |text, line| {
            line.map(|line| format!("{text}{}- {line}\n", " ".repeat(indent)))
        })
}

/// A section that does not exist, added at the end of the file.
fn append_section(
    text: &str,
    section: &str,
    key: &str,
    values: &[String],
) -> Result<String, Refused> {
    let separator = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let head = format!("{section}:\n{STEP}{key}:\n");
    Ok(format!(
        "{text}{separator}{}",
        block(&head, STEP.len() * 2, values)?
    ))
}

/// A block list under the key at line `k`: the values go after its last item.
fn block_list(
    lines: &[&str],
    k: usize,
    end: usize,
    key_indent: usize,
    values: &[String],
) -> Result<String, Refused> {
    let is_item = |line: &str| {
        let trimmed = line.trim_start();
        trimmed == "-"
            || trimmed.starts_with("- ")
            || trimmed.starts_with("-\n")
            || trimmed.starts_with("-\r")
    };
    let items: Vec<(usize, &str)> = lines
        .iter()
        .copied()
        .enumerate()
        .take(end)
        .skip(k + 1)
        .take_while(|(_, line)| {
            is_blank_or_comment(line)
                || indent_of(line) > key_indent
                || (indent_of(line) == key_indent && is_item(line))
        })
        .filter(|(_, line)| !is_blank_or_comment(line))
        .collect();
    let item_indent = items
        .first()
        .map_or(key_indent + STEP.len(), |(_, line)| indent_of(line));
    if items.first().is_some_and(|(_, line)| !is_item(line)) {
        return Err(Refused::Form);
    }
    let after = items.last().map_or(k, |(index, _)| *index);
    Ok(insert_after(lines, after, &block("", item_indent, values)?))
}

/// A flow list on the key's line: `, <value>` goes before its `]`.
fn flow_list(lines: &[&str], k: usize, value: &str, values: &[String]) -> Result<String, Refused> {
    let line = lines.get(k).ok_or(Refused::Form)?;
    let list = value.trim();
    let inner = list
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or(Refused::Form)?;
    if inner.contains(['#', '[', ']', '{', '}', '&', '*', '!']) {
        return Err(Refused::Form);
    }
    let close = line.rfind(']').ok_or(Refused::Form)?;
    let written = values
        .iter()
        .map(|value| scalar(value, true).ok_or(Refused::Form))
        .collect::<Result<Vec<_>, _>>()?;
    let joined = written.join(", ");
    let added = if inner.trim().is_empty() {
        joined
    } else {
        format!(", {joined}")
    };
    let (before, after) = line.split_at_checked(close).ok_or(Refused::Form)?;
    Ok(lines
        .iter()
        .enumerate()
        .map(|(index, other)| {
            if index == k {
                format!("{before}{added}{after}")
            } else {
                (*other).to_owned()
            }
        })
        .collect())
}

/// The lines with `new` inserted after line `after`, which gets a line feed when it lacks one.
fn insert_after(lines: &[&str], after: usize, new: &str) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == after && !line.ends_with('\n') {
                format!(
                    "{line}\n{}",
                    if index + 1 == lines.len() { new } else { "" }
                )
            } else if index == after {
                format!("{line}{new}")
            } else {
                (*line).to_owned()
            }
        })
        .collect()
}
