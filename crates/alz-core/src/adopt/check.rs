//! The reader check of a plan (spec §4.3, TASK-73): the repo layer as the plan would leave it goes
//! through the same merge every layer goes through, with the manifest as it would be edited, so a
//! plan never holds text that reading the layer would refuse.

use crate::diagnostic::{Level, Message};
use crate::layer::LayerContents;
use crate::merge::{Placements, merge};
use crate::path::SafeRelPath;
use crate::stack::{Manifest, ResolvedLayer, ResolvedStack};
use crate::untrusted_text::Stripped;

use super::draft::Draft;

/// Adds a problem to `draft` for each error that merging the stack with the planned repo layer
/// finds. The problem names the native file the refused layer file came from and prints the
/// reader's own line, which holds no raw text.
pub(super) fn read_back(
    stack: &ResolvedStack,
    manifest: &Manifest,
    placements: &Placements,
    draft: &mut Draft<'_>,
) {
    let Some((last, general)) = stack.layers().split_last() else {
        return;
    };
    let planned = ResolvedLayer::new(
        last.source().clone(),
        last.meta().clone(),
        planned_contents(last.contents(), draft),
        last.pin().cloned(),
    );
    let layers: Vec<ResolvedLayer> = general.iter().cloned().chain([planned]).collect();
    let Ok(stack) = ResolvedStack::validate(layers) else {
        draft.problem(
            draft.ctx.layer_dir.as_str().to_owned().as_str(),
            Message::new("the planned stack is refused"),
        );
        return;
    };
    let (_, found) = merge(&stack, manifest, placements).into_parts();
    for error in found.iter().filter(|found| found.level() == Level::Error) {
        let line = error.to_string();
        let file = source_of(&line, draft);
        draft.problem(
            &file,
            Message::new("the repo layer that adoption plans would be refused: ")
                .then(Stripped::from_untrusted(&line)),
        );
    }
}

/// The files of the repo layer after the plan: what it has, the planned files over them, and the
/// files moved in.
fn planned_contents(existing: &LayerContents, draft: &Draft<'_>) -> LayerContents {
    let executable: Vec<SafeRelPath> = existing
        .paths()
        .filter(|path| existing.is_executable(path))
        .cloned()
        .chain(
            draft
                .moves
                .iter()
                .filter(|moved| moved.file.is_executable())
                .map(|moved| moved.to.clone()),
        )
        .collect();
    existing
        .iter()
        .map(|(path, bytes)| (path.clone(), bytes.to_vec()))
        .chain(draft.layer_files())
        .chain(
            draft
                .moves
                .iter()
                .map(|moved| (moved.to.clone(), moved.file.bytes().to_vec())),
        )
        .collect::<LayerContents>()
        .with_executable(executable)
}

/// The native file behind the layer file a reader line is about; the layer's folder when the line
/// names no planned file.
fn source_of(line: &str, draft: &Draft<'_>) -> String {
    let planned = draft
        .files
        .iter()
        .map(|(path, (from, _))| (path.as_str(), from.as_str()))
        .chain(
            draft
                .moves
                .iter()
                .map(|moved| (moved.to.as_str(), moved.from.as_str())),
        );
    planned
        .filter(|(path, _)| line.contains(&format!(":{path}")))
        .max_by_key(|(path, _)| path.len())
        .map_or_else(
            || draft.ctx.layer_dir.as_str().to_owned(),
            |(_, from)| from.to_owned(),
        )
}
