//! A stack whose layers are resolved (spec §6): each entry's directory found, read and parsed,
//! and the stack validated as a whole (§6 step 2). Finding and reading the directories is
//! `alz-io`'s; everything here is a decision over the values it gives.

use crate::diagnostic::Diagnostics;
use crate::layer::LayerContents;
use crate::layer::LayerMeta;
use crate::lock::{LayerFiles, ObjectId};
use crate::stack::GitRef;
use crate::stack::source::Source;
use crate::stack::validate;
use crate::untrusted_text::Stripped;

/// What `layer.lock` records of a layer that came from git: the `rev` and the commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// `rev`: the first 12 characters of the commit for a `git:` or `github:` layer, the
    /// manifest's `ref` for a pinned `path:` layer.
    pub rev: GitRef,
    /// The commit.
    pub sha: ObjectId,
}

/// One stack entry resolved: where it came from, what `layer.yaml` says and the files it holds,
/// with the bytes the walk read and the content hash covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLayer {
    source: Source,
    meta: LayerMeta,
    contents: LayerContents,
    files: LayerFiles,
    pin: Option<Pin>,
}

impl ResolvedLayer {
    /// A resolved layer. `pin` is `None` for an unpinned `path:` layer, which is read in place.
    #[must_use]
    pub fn new(source: Source, meta: LayerMeta, contents: LayerContents, pin: Option<Pin>) -> Self {
        Self {
            source,
            meta,
            files: contents.files(),
            contents,
            pin,
        }
    }

    /// The stack entry's source.
    #[must_use]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    /// What the layer says about itself.
    #[must_use]
    pub const fn meta(&self) -> &LayerMeta {
        &self.meta
    }

    /// The files the content hash covers, with the digest of each file's bytes: derived from
    /// [`ResolvedLayer::contents`], so it is what the lock check covered.
    #[must_use]
    pub const fn files(&self) -> &LayerFiles {
        &self.files
    }

    /// The files the content hash covers, with the bytes the walk read. The merge reads these
    /// bytes and never the file system, so it uses exactly what the lock check covered.
    #[must_use]
    pub const fn contents(&self) -> &LayerContents {
        &self.contents
    }

    /// The pin, for a layer that came from git.
    #[must_use]
    pub const fn pin(&self) -> Option<&Pin> {
        self.pin.as_ref()
    }
}

/// A stack that passed §6 step 2: no name is used twice and the visibility only grows along it.
/// It is never empty, because a manifest's stack is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStack(Vec<ResolvedLayer>);

impl ResolvedStack {
    /// Validates the layers in stack order (§6 step 2): every problem at once, in this order,
    /// the duplicate names and then the visibility order.
    ///
    /// # Errors
    ///
    /// Every problem found, as diagnostics.
    pub fn validate(layers: Vec<ResolvedLayer>) -> Result<Self, Diagnostics> {
        Self::check(layers).map_err(|(_, problems)| problems)
    }

    /// As [`validate`](Self::validate), and gives the layers back with the problems when the stack
    /// is not valid.
    fn check(layers: Vec<ResolvedLayer>) -> Result<Self, (Vec<ResolvedLayer>, Diagnostics)> {
        let problems =
            validate::duplicate_names(&layers).merge(validate::visibility_order(&layers));
        if problems.has_errors() {
            Err((layers, problems))
        } else {
            Ok(Self(layers))
        }
    }

    /// The layers, most general first.
    #[must_use]
    pub fn layers(&self) -> &[ResolvedLayer] {
        &self.0
    }

    fn into_layers(self) -> Vec<ResolvedLayer> {
        self.0
    }
}

/// An entry that was resolved, and what was reported while it was: a `.env` file is an error that
/// does not stop the entry, a cache index that cannot be written is a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEntry {
    /// The layer.
    pub layer: ResolvedLayer,
    /// Errors and warnings found on the way.
    pub diagnostics: Diagnostics,
}

/// An entry that could not be resolved, with the problems that say why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedEntry {
    /// The source as written, stripped: what the summary of `alz lint` calls the layer.
    pub source: Stripped,
    /// The problems, which may be none when an earlier entry already reported the cause (a git
    /// that is too old, a cache that cannot be used).
    pub diagnostics: Diagnostics,
}

/// What resolving the whole stack found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackReport {
    names: Vec<Stripped>,
    diagnostics: Diagnostics,
    layers: ReportedLayers,
}

/// The layers that were resolved: a stack that passed every check, or only the layers, kept so
/// that a command can still look at each of them.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReportedLayers {
    Valid(ResolvedStack),
    Unusable(Vec<ResolvedLayer>),
}

impl StackReport {
    /// The report for the entries in stack order: each entry's diagnostics in turn, then those of
    /// §6 step 2 over the entries that were resolved. The stack is there only when every entry was
    /// resolved and the validation found nothing wrong.
    #[must_use]
    pub fn of(entries: Vec<Result<ResolvedEntry, FailedEntry>>) -> Self {
        let complete = entries.iter().all(Result::is_ok);
        let names = entries
            .iter()
            .map(|entry| match entry {
                Ok(resolved) => Stripped::from_untrusted(resolved.layer.meta().name().as_str()),
                Err(failed) => failed.source.clone(),
            })
            .collect();
        let (layers, reported) = entries.into_iter().fold(
            (Vec::new(), Diagnostics::new()),
            |(mut layers, reported), entry| match entry {
                Ok(resolved) => {
                    layers.push(resolved.layer);
                    (layers, reported.merge(resolved.diagnostics))
                }
                Err(failed) => (layers, reported.merge(failed.diagnostics)),
            },
        );
        let (layers, diagnostics) = match ResolvedStack::check(layers) {
            Ok(stack) if complete && !reported.has_errors() => {
                (ReportedLayers::Valid(stack), reported)
            }
            Ok(stack) => (ReportedLayers::Unusable(stack.into_layers()), reported),
            Err((layers, problems)) => (ReportedLayers::Unusable(layers), reported.merge(problems)),
        };
        Self {
            names,
            diagnostics,
            layers,
        }
    }

    /// What each layer is called in the summary: its name, or its source as written when the
    /// entry could not be resolved. One for each entry, in stack order.
    #[must_use]
    pub fn names(&self) -> &[Stripped] {
        &self.names
    }

    /// Every diagnostic, in the order found.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The validated stack, when there is one.
    #[must_use]
    pub const fn stack(&self) -> Option<&ResolvedStack> {
        match &self.layers {
            ReportedLayers::Valid(stack) => Some(stack),
            ReportedLayers::Unusable(_) => None,
        }
    }

    /// The layers that were resolved, in stack order, whether or not the stack is valid: an entry
    /// that failed is not among them. A command that checks each layer alone (`alz lint`) reads
    /// them here.
    #[must_use]
    pub fn resolved_layers(&self) -> &[ResolvedLayer] {
        match &self.layers {
            ReportedLayers::Valid(stack) => stack.layers(),
            ReportedLayers::Unusable(layers) => layers,
        }
    }
}

#[cfg(test)]
mod tests;
