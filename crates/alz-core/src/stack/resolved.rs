//! A stack whose layers are resolved (spec §6): each entry's directory found, read and parsed,
//! and the stack validated as a whole (§6 step 2). Finding and reading the directories is
//! `alz-io`'s; everything here is a decision over the values it gives.

use crate::diagnostic::Diagnostics;
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

/// One stack entry resolved: where it came from, what `layer.yaml` says and which files it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLayer {
    source: Source,
    meta: LayerMeta,
    files: LayerFiles,
    pin: Option<Pin>,
}

impl ResolvedLayer {
    /// A resolved layer. `pin` is `None` for an unpinned `path:` layer, which is read in place.
    #[must_use]
    pub const fn new(source: Source, meta: LayerMeta, files: LayerFiles, pin: Option<Pin>) -> Self {
        Self {
            source,
            meta,
            files,
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

    /// The files the content hash covers.
    #[must_use]
    pub const fn files(&self) -> &LayerFiles {
        &self.files
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
        let problems =
            validate::duplicate_names(&layers).merge(validate::visibility_order(&layers));
        if problems.has_errors() {
            Err(problems)
        } else {
            Ok(Self(layers))
        }
    }

    /// The layers, most general first.
    #[must_use]
    pub fn layers(&self) -> &[ResolvedLayer] {
        &self.0
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
    stack: Option<ResolvedStack>,
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
        match ResolvedStack::validate(layers) {
            Ok(stack) => Self {
                names,
                stack: complete.then_some(stack).filter(|_| !reported.has_errors()),
                diagnostics: reported,
            },
            Err(problems) => Self {
                names,
                diagnostics: reported.merge(problems),
                stack: None,
            },
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
        self.stack.as_ref()
    }
}

#[cfg(test)]
mod tests;
