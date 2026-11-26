//! Passthrough files joining the generated tree (spec §7.11): each file of `targets/<target>/<path>`
//! goes to `<path>`, byte for byte with its executable bit, after every generated file.
//!
//! A passthrough path that a file system would make one file with a path already in the tree,
//! compared after case folding (§4.3), is the error `passthrough` at the layer file when the
//! path is a generated file's, and the generated file is kept. Two passthrough files that
//! collide are the error `collision`, and the first stays.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::item::{PassthroughFile, place};
use crate::merge::{MergedPassthrough, Replace, Traced};
use crate::path::SafeRelPath;
use crate::render::tree::{
    FileMode, FileTree, GeneratedFile, OutputPath, OutputPathError, PathCollision,
};
use crate::untrusted_text::Stripped;
use alz_adapter::TargetId;

/// `tree` with the passthrough files of `target` added, and the errors of the files that cannot be.
pub(crate) fn join(
    tree: FileTree,
    files: &MergedPassthrough,
    target: &TargetId,
) -> (FileTree, Diagnostics) {
    let generated = tree.clone();
    let merged = files.get(target).into_iter().flat_map(Replace::iter);
    merged.fold(
        (tree, Diagnostics::new()),
        |(mut tree, found), (path, file)| {
            let outcome = OutputPath::try_from(path.clone())
                .map_err(|error| unprintable(error, path, file))
                .and_then(|output| {
                    tree.insert(output, generated_file(file.value()))
                        .map_err(|collision| refused(&collision, &generated, file))
                });
            match outcome {
                Ok(()) => (tree, found),
                Err(problem) => (tree, found.with(problem)),
            }
        },
    )
}

fn generated_file(file: &PassthroughFile) -> GeneratedFile {
    GeneratedFile {
        bytes: file.bytes().to_vec(),
        mode: if file.is_executable() {
            FileMode::Executable
        } else {
            FileMode::Regular
        },
    }
}

/// The place of the layer file that won the path.
fn at(file: &Traced<PassthroughFile>) -> Option<crate::diagnostic::Where> {
    file.provenance()
        .contributions()
        .last()
        .map(|last| place(last.layer(), file.value().source()))
}

fn refused(
    collision: &PathCollision,
    generated: &FileTree,
    file: &Traced<PassthroughFile>,
) -> Diagnostic {
    if generated.claim_of(collision.rejected()).is_some() {
        Diagnostic::error(
            Code::Passthrough,
            at(file),
            Message::new("passthrough ")
                .then(Stripped::from_untrusted(collision.rejected().as_str()))
                .then(" collides with a generated file"),
        )
    } else {
        Diagnostic::error(
            Code::Collision,
            at(file),
            Message::new("").then(Stripped::from_untrusted(&collision.to_string())),
        )
    }
}

fn unprintable(
    error: OutputPathError,
    path: &SafeRelPath,
    file: &Traced<PassthroughFile>,
) -> Diagnostic {
    Diagnostic::error(
        Code::Layer,
        at(file),
        Message::new("the passthrough path ")
            .then(Stripped::from_untrusted(path.as_str()))
            .then(": ")
            .then(Stripped::from_untrusted(&error.to_string())),
    )
}
