//! What an adapter returned, written as the core's files (spec §8.0 **Output.**, §4.3): JSON by the
//! stable JSON emitter, Markdown with the header that only the core writes, bytes as they are.
//!
//! [`emit`] is pure and total. An adapter's output is untrusted until it is written, and the core
//! checks all of it first (§8.0 **Output checks.**): each path must be a safe relative path in the
//! adapter's territory that the core does not write itself, its sources must stand in a comment,
//! its values must be ones the emitters write, the output must stay within the limits, its
//! diagnostic codes must be its own, and its coverage report must account for every security item
//! of the model. Each failure is a diagnostic, the error `target` (`layer` for a source), and the
//! file is left out, so the build with an error writes nothing.

mod coverage;
mod json;
mod limits;
mod markdown;
mod paths;

use alz_adapter::{
    Content, Descriptor, Diagnostic as Returned, Level as ReturnedLevel, Output, OutputFile,
    SecurityItems, TargetId,
};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::render::tree::{FileMode, FileTree, GeneratedFile, OutputPath};
use crate::untrusted_text::Stripped;
use limits::Limits;

/// The files and the diagnostics of one adapter's output, once every check of §8.0 has run.
///
/// `descriptor` is the adapter's own, for its territory, and `security` holds the security items
/// of the model the adapter rendered, which its coverage report must account for. The files are
/// the output's files in the order given; two files at one path are the error `collision` and the
/// first stays (§4.3).
#[must_use]
pub fn emit(
    descriptor: &Descriptor,
    security: &SecurityItems,
    output: &Output,
) -> (FileTree, Diagnostics) {
    emit_within(&Limits::SPEC, descriptor, security, output)
}

/// [`emit`] with the limits given, so that a test can break them without a quarter of a gigabyte.
fn emit_within(
    limits: &Limits,
    descriptor: &Descriptor,
    security: &SecurityItems,
    output: &Output,
) -> (FileTree, Diagnostics) {
    let target = &descriptor.target;
    let checked = output
        .diagnostics
        .iter()
        .map(|returned| diagnostic(target, returned))
        .collect::<Diagnostics>()
        .merge(coverage::check(target, security, &output.coverage));
    limits.check_count(output.files.len()).map_or_else(
        |over| {
            (
                FileTree::new(),
                checked.clone().merge(refused(target, over)),
            )
        },
        |()| {
            let (tree, found) = output.files.iter().fold(
                (FileTree::new(), checked.clone()),
                |(tree, found), file| match written(limits, descriptor, file) {
                    Ok((path, generated)) => insert(tree, found, path, generated),
                    Err(problems) => (tree, found.merge(problems)),
                },
            );
            let sizes = tree.iter().map(|(_, file)| file.bytes.len());
            match limits.check_total(sizes) {
                Ok(()) => (tree, found),
                Err(over) => (tree, found.merge(refused(target, over))),
            }
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
    limits: &Limits,
    descriptor: &Descriptor,
    file: &OutputFile,
) -> Result<(OutputPath, GeneratedFile), Diagnostics> {
    let target = &descriptor.target;
    let path = OutputPath::try_from(file.path.as_str()).map_err(|refusal| {
        refused(
            target,
            Message::new("the path ")
                .then(Stripped::from_untrusted(&file.path))
                .then(" is not allowed: ")
                .then(Stripped::from_untrusted(&refusal.to_string())),
        )
    })?;
    paths::check(&descriptor.territory, &path)
        .map_err(|refusal| refused(target, refusal.rule(&path)))?;
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
    limits.check_file(generated.bytes.len()).map_err(|over| {
        refused(
            target,
            Message::new("the file ")
                .then(Stripped::from_untrusted(path.as_str()))
                .then(" is too large: ")
                .and(over),
        )
    })?;
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
/// message is stripped, and a code that is not one of the adapter's codes is the error `target`.
fn diagnostic(target: &TargetId, returned: &Returned) -> Diagnostic {
    let at = returned.file.as_ref().map(|source| {
        Where::layer_file(
            Stripped::from_untrusted(source.layer.as_str()),
            Stripped::from_untrusted(&source.path),
        )
    });
    let message = Message::new(Stripped::from_untrusted(&returned.message));
    match (code_of(target, &returned.code), returned.level) {
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

/// The codes that every adapter may raise (§10.1 **Adapter codes.**).
const SHARED_CODES: [Code; 3] = [Code::Models, Code::ToolScope, Code::HookOnFail];

/// The own codes of the adapters built into alz, which the tables list (§10.1): each is the id of
/// its adapter, `-` and a word.
const OWN_CODES: [Code; 1] = [Code::ClaudeAgent];

/// The code of the tables for `code`, when it is one of the adapter's codes: a shared adapter code,
/// or an own code that starts with the adapter's id and `-`. A code that the tables give to the
/// core is never an adapter's, whatever it starts with.
fn code_of(target: &TargetId, code: &str) -> Option<Code> {
    SHARED_CODES
        .into_iter()
        .chain(
            OWN_CODES
                .into_iter()
                .filter(|own| in_namespace(target, own.as_str())),
        )
        .find(|known| known.as_str() == code)
}

/// Whether `code` is the adapter's id, `-` and then something more.
fn in_namespace(target: &TargetId, code: &str) -> bool {
    code.strip_prefix(target.as_str())
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|word| !word.is_empty())
}

#[cfg(test)]
mod tests;
