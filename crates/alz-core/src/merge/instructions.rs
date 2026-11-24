//! The merge of instructions (spec §7.1).
//!
//! The blocks of all layers are concatenated in stack order, and within a layer by file name in
//! code point order (§6). A file of a more specific layer with `mode: replace` or `mode: remove`
//! acts on the blocks of more general layers with the same file name: `remove` drops them, and
//! `replace` drops them and puts the file's own block in their place in the order of the layer
//! that replaced it, so the block is tagged with that layer. A file without `mode` adds a block,
//! also when a more general layer has a file with the same name.
//!
//! A block that is empty after trimming is dropped when the merge is finished, so an empty file
//! can still be replaced or removed by a more specific layer.

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::item::{ClaudeExtra, InstructionEdit, InstructionFile, InstructionItems, place};
use crate::layer::{ItemPath, LayerHeading, LayerMeta, LayerName};
use crate::merge::action::Action;
use crate::merge::provenance::{Origin, Traced};
use crate::merge::stack::LayerContext;
use crate::path::SafeRelPath;
use crate::stack::ResolvedLayer;
use crate::untrusted_text::Stripped;

/// One block of the composed `AGENTS.md`: a file of a layer, under the layer's heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionBlock {
    heading: LayerHeading,
    origin: Origin,
    file: SafeRelPath,
    body: String,
}

impl InstructionBlock {
    /// The heading that wraps the block, `## <Layer kind>: <layer name>`. Blocks of one layer
    /// that follow each other share one heading when the renderer composes them (§7.1).
    #[must_use]
    pub const fn heading(&self) -> &LayerHeading {
        &self.heading
    }

    /// Where the block comes from: the layer that made it, with its revision, and the file. For a
    /// block a layer replaced, that is the layer that replaced it.
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The file the block comes from, `instructions/<name>.md`.
    #[must_use]
    pub const fn file(&self) -> &SafeRelPath {
        &self.file
    }

    /// The text, trimmed and not empty.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// The Claude-specific text of one layer, `targets/claude/CLAUDE.extra.md` (§7.1), with where it
/// comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedClaudeExtra {
    text: ClaudeExtra,
    origin: Origin,
}

impl MergedClaudeExtra {
    /// The layer the text comes from.
    #[must_use]
    pub const fn layer(&self) -> &LayerName {
        self.text.layer()
    }

    /// The text, trimmed and not empty.
    #[must_use]
    pub fn body(&self) -> &str {
        self.text.body()
    }

    /// Where the text comes from: the layer, its revision and the file.
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }
}

/// The state of the fold for instructions: the blocks so far, and the Claude text of each layer.
#[derive(Debug, Default)]
pub(super) struct InstructionState {
    blocks: Vec<Traced<InstructionBlock>>,
    claude_extras: Vec<MergedClaudeExtra>,
}

/// The finished merge: the blocks with their provenance, and the Claude text of each layer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MergedInstructions {
    pub(super) blocks: Vec<Traced<InstructionBlock>>,
    pub(super) claude_extras: Vec<MergedClaudeExtra>,
}

impl InstructionState {
    /// The state after one more, more specific, layer.
    pub(super) fn absorb(self, context: &LayerContext<'_>) -> (Self, Diagnostics) {
        let (items, read) = InstructionItems::read(context.layer());
        let (blocks, found) =
            items
                .files()
                .iter()
                .fold((self.blocks, read), |(blocks, found), file| {
                    let (blocks, problem) = apply(blocks, context.layer(), file);
                    (blocks, found.merge(problem.into_iter().collect()))
                });
        (
            Self {
                blocks,
                claude_extras: self
                    .claude_extras
                    .into_iter()
                    .chain(items.claude_extra().and_then(|text| {
                        Origin::of_item(context.layer(), ItemPath::ClaudeExtra).map(|origin| {
                            MergedClaudeExtra {
                                text: text.clone(),
                                origin,
                            }
                        })
                    }))
                    .collect(),
            },
            found,
        )
    }

    /// The blocks that have text, and the Claude text.
    pub(super) fn finish(self) -> MergedInstructions {
        MergedInstructions {
            blocks: self
                .blocks
                .into_iter()
                .filter(|block| !block.value().body.is_empty())
                .collect(),
            claude_extras: self.claude_extras,
        }
    }
}

/// The blocks after one file of the layer `meta` describes, and the problem when a `replace` or
/// `remove` has no block to act on.
fn apply(
    blocks: Vec<Traced<InstructionBlock>>,
    layer: &ResolvedLayer,
    file: &InstructionFile,
) -> (Vec<Traced<InstructionBlock>>, Option<Diagnostic>) {
    let meta = layer.meta();
    let origin = Origin::new(layer, file.file().clone());
    let block = |body: &str| InstructionBlock {
        heading: LayerHeading::of(meta),
        origin: origin.clone(),
        file: file.file().clone(),
        body: body.to_owned(),
    };
    match file.edit() {
        InstructionEdit::Add(body) => (
            blocks
                .into_iter()
                .chain([Traced::new(block(body), origin, Action::Define)])
                .collect(),
            None,
        ),
        InstructionEdit::Replace(body) => {
            let (acted_on, rest) = split_off(blocks, file.file());
            match acted_on.into_iter().last() {
                Some(previous) => (
                    rest.into_iter()
                        .chain([previous.revised(block(body), origin, Action::Replace)])
                        .collect(),
                    None,
                ),
                None => (rest, Some(unresolved("replace", meta, file))),
            }
        }
        InstructionEdit::Remove => {
            let (acted_on, rest) = split_off(blocks, file.file());
            let problem = acted_on
                .is_empty()
                .then(|| unresolved("remove", meta, file));
            (rest, problem)
        }
    }
}

/// The blocks of the file `path`, and the others, each in its order.
fn split_off(
    blocks: Vec<Traced<InstructionBlock>>,
    path: &SafeRelPath,
) -> (Vec<Traced<InstructionBlock>>, Vec<Traced<InstructionBlock>>) {
    blocks
        .into_iter()
        .partition(|block| block.value().file == *path)
}

fn unresolved(mode: &'static str, meta: &LayerMeta, file: &InstructionFile) -> Diagnostic {
    Diagnostic::error(
        Code::Unresolved,
        Some(place(meta.name(), file.file())),
        Message::new("mode: ")
            .then(mode)
            .then(", but no more general layer has ")
            .then(Stripped::from_untrusted(file.file().as_str())),
    )
}

#[cfg(test)]
mod tests;
