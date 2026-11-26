//! What an adapter returned, written as the core's files (spec §8.0 **Output.**, §4.3): JSON by the
//! stable JSON emitter, Markdown with the header that only the core writes, bytes as they are.
//!
//! [`emit`] is pure and total. An adapter's file is untrusted until it is written: its path must be
//! a safe relative path, its sources must stand in a comment, and its values must be ones the
//! emitters write. Each failure is a diagnostic and the file is left out, so the build with an
//! error writes nothing.

mod json;
mod markdown;

use alz_adapter::TargetId;
use alz_adapter::{Content, Diagnostic as Returned, Level as ReturnedLevel, Output, OutputFile};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::render::tree::{FileMode, FileTree, GeneratedFile, OutputPath};
use crate::untrusted_text::Stripped;

/// The files and the diagnostics of one adapter's output.
///
/// The files are the output's files in the order given; two files at one path are the error
/// `collision` and the first stays (§4.3).
#[must_use]
pub fn emit(target: &TargetId, output: &Output) -> (FileTree, Diagnostics) {
    let converted = output
        .diagnostics
        .iter()
        .map(|returned| diagnostic(target, returned))
        .collect::<Diagnostics>();
    output.files.iter().fold(
        (FileTree::new(), converted),
        |(tree, found), file| match written(target, file) {
            Ok((path, generated)) => insert(tree, found, path, generated),
            Err(problems) => (tree, found.merge(problems)),
        },
    )
}

/// `tree` with the file added, or the error `collision` when its path is the same file as one
/// already there.
fn insert(
    mut tree: FileTree,
    found: Diagnostics,
    path: OutputPath,
    file: GeneratedFile,
) -> (FileTree, Diagnostics) {
    match tree.insert(path, file) {
        Ok(()) => (tree, found),
        Err(collision) => (
            tree,
            found.with(Diagnostic::error(
                Code::Collision,
                None,
                Message::new("").then(Stripped::from_untrusted(&collision.to_string())),
            )),
        ),
    }
}

/// One file as the core writes it, or why it cannot be.
fn written(
    target: &TargetId,
    file: &OutputFile,
) -> Result<(OutputPath, GeneratedFile), Diagnostics> {
    let path = OutputPath::try_from(file.path.as_str()).map_err(|refusal| {
        refused(
            target,
            Message::new("the path ")
                .then(Stripped::from_untrusted(&file.path))
                .then(" is not allowed: ")
                .then(Stripped::from_untrusted(&refusal.to_string())),
        )
    })?;
    let generated = match &file.content {
        Content::Json(value) => json::file(value),
        Content::Markdown(body) => markdown::file(target, &path, body)?,
        Content::Toml(_) => {
            return Err(refused(
                target,
                Message::new("the file ")
                    .then(Stripped::from_untrusted(path.as_str()))
                    .then(" holds TOML, which this alz does not write yet"),
            ));
        }
        Content::Bytes(bytes) => GeneratedFile {
            bytes: bytes.content.clone(),
            mode: if bytes.executable {
                FileMode::Executable
            } else {
                FileMode::Regular
            },
        },
    };
    Ok((path, generated))
}

/// The error `target`, naming the adapter and the rule it broke (§8.0 **Output checks.**).
fn refused(target: &TargetId, rule: Message) -> Diagnostics {
    Diagnostics::new().with(Diagnostic::error(
        Code::Target,
        None,
        Message::new("the '")
            .then(MessagePart::printable(target))
            .then("' adapter returned output that is refused: ")
            .and(rule),
    ))
}

/// The core's diagnostic for one the adapter returned: the level and place are the adapter's, the
/// message is stripped, and a code that is not one of the adapter codes is the error `target`.
fn diagnostic(target: &TargetId, returned: &Returned) -> Diagnostic {
    let at = returned.file.as_ref().map(|source| {
        Where::layer_file(
            Stripped::from_untrusted(source.layer.as_str()),
            Stripped::from_untrusted(&source.path),
        )
    });
    let message = Message::new(Stripped::from_untrusted(&returned.message));
    match (code_of(&returned.code), returned.level) {
        (Some(code), ReturnedLevel::Error) => Diagnostic::error(code, at, message),
        (Some(code), ReturnedLevel::Warning) => Diagnostic::warning(code, at, message),
        (Some(code), ReturnedLevel::Note) => Diagnostic::note(code, at, message),
        (None, _) => Diagnostic::error(
            Code::Target,
            None,
            Message::new("the '")
                .then(MessagePart::printable(target))
                .then("' adapter returned the diagnostic code ")
                .then(Stripped::from_untrusted(&returned.code))
                .then(", which is not one of its codes"),
        ),
    }
}

/// The code of the tables for an adapter's code (§10.1 **Adapter codes.**). The tables list the
/// codes of the adapters built into alz.
fn code_of(code: &str) -> Option<Code> {
    [
        Code::Models,
        Code::ToolScope,
        Code::HookOnFail,
        Code::ClaudeAgent,
    ]
    .into_iter()
    .find(|known| known.as_str() == code)
}

#[cfg(test)]
mod tests;
