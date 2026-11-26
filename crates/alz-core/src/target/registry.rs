//! The registry of adapters (spec §8.0 **Rendering.**): the adapters a build may use, by target id.
//!
//! The `alz` binary chooses the adapters it registers; [`Registry::builtin`] is the choice of the
//! binary that ships: `claude`. An adapter built for another major version of the model, or for a
//! later minor version than the core's, is refused when it is registered, and so is a second
//! adapter for one target. Each target of `targets` is rendered by its own adapter, in the order
//! of `targets` (§8.0).

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::LazyLock;

use alz_adapter::{
    Adapter, Context, Descriptor, MODEL_VERSION, Model, ModelVersion, NativeVocabulary, TargetId,
};
use thiserror::Error;

use crate::diagnostic::Diagnostics;
use crate::diagnostic::{Code, Diagnostic, Message, MessagePart, Where};
use crate::item::KeyRules;
use crate::render::adapter_output;
use crate::render::tree::FileTree;
use crate::target::codex;
use crate::untrusted_text::Stripped;
use alz_target_claude::ClaudeAdapter;

/// The target ids that alz names and that have no adapter yet (§8 **Targets without an
/// adapter**), each with the native vocabulary that `settings.yaml` is validated with until the
/// adapter's descriptor states it (TASK-110). A descriptor-only entry: key rules and no renderer.
static PLANNED: LazyLock<BTreeMap<TargetId, NativeVocabulary>> =
    LazyLock::new(|| BTreeMap::from([(crate::target::codex().clone(), codex::vocabulary())]));

/// Why an adapter cannot be registered.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegisterError {
    /// The adapter is built for a model version that this alz cannot run.
    #[error("the adapter for '{target}' is built for model {built}, which model {core} cannot run")]
    ModelVersion {
        /// The adapter's target.
        target: TargetId,
        /// The version the adapter is built for.
        built: ModelVersion,
        /// The version of the core.
        core: ModelVersion,
    },
    /// Another adapter is registered for the target.
    #[error("an adapter for '{0}' is registered already")]
    Twice(TargetId),
}

/// The adapters of a build, by target id.
#[derive(Default)]
pub struct Registry {
    adapters: BTreeMap<TargetId, Box<dyn Adapter>>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.adapters.keys())
            .finish()
    }
}

impl Registry {
    /// A registry without adapters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The adapters built into alz: `claude`.
    #[must_use]
    pub fn builtin() -> Self {
        // The built-in adapter is built for the model of this core, so it is never refused.
        Self::new()
            .with(Box::new(ClaudeAdapter::new()))
            .unwrap_or_default()
    }

    /// The registry with `adapter` added.
    ///
    /// # Errors
    ///
    /// [`RegisterError::ModelVersion`] when the adapter is built for another major version of the
    /// model or a later minor one, and [`RegisterError::Twice`] when its target has an adapter.
    pub fn with(mut self, adapter: Box<dyn Adapter>) -> Result<Self, RegisterError> {
        let descriptor = adapter.descriptor();
        let target = descriptor.target.clone();
        if MODEL_VERSION.runs(descriptor.model_version) {
            match self.adapters.entry(target) {
                Entry::Occupied(taken) => Err(RegisterError::Twice(taken.key().clone())),
                Entry::Vacant(free) => {
                    free.insert(adapter);
                    Ok(self)
                }
            }
        } else {
            Err(RegisterError::ModelVersion {
                target,
                built: descriptor.model_version,
                core: MODEL_VERSION,
            })
        }
    }

    /// The adapter of `target`, when one is registered.
    #[must_use]
    pub fn adapter(&self, target: &TargetId) -> Option<&dyn Adapter> {
        self.adapters.get(target).map(AsRef::as_ref)
    }

    /// The descriptor of the adapter of `target`, when one is registered.
    #[must_use]
    pub fn descriptor(&self, target: &TargetId) -> Option<&Descriptor> {
        self.adapter(target).map(Adapter::descriptor)
    }

    /// The key rules of `target`'s native settings (§7.9): those of its adapter's descriptor, or
    /// for a target that alz names and that has no adapter yet, the descriptor-only entry of the
    /// core. `None` for any other target, whose section `settings.yaml` reads as a mapping and
    /// nothing more.
    #[must_use]
    pub fn key_rules(&self, target: &TargetId) -> Option<KeyRules<'_>> {
        self.descriptor(target)
            .map(|descriptor| KeyRules::new(&descriptor.target, &descriptor.vocabulary))
            .or_else(|| {
                PLANNED
                    .get_key_value(target)
                    .map(|(planned, vocabulary)| KeyRules::new(planned, vocabulary))
            })
    }

    /// The files and diagnostics of `target`: its adapter renders the model, and the core checks
    /// everything it returned (§8.0 **Output checks.**) before it emits the files. `None` when no
    /// adapter is registered for the target.
    #[must_use]
    pub fn render(
        &self,
        target: &TargetId,
        model: &Model,
        context: &Context,
    ) -> Option<(FileTree, Diagnostics)> {
        self.adapter(target).map(|adapter| {
            let output = adapter.render(model, context);
            adapter_output::emit(adapter.descriptor(), &model.security, &output)
        })
    }

    /// The registered target ids, in name order.
    pub fn targets(&self) -> impl Iterator<Item = &TargetId> + '_ {
        self.adapters.keys()
    }

    /// The files that adoption always takes, when they exist: those of every registered adapter,
    /// in target order (§4.3).
    #[must_use]
    pub fn adopted_files(&self) -> Vec<&str> {
        self.adapters
            .values()
            .flat_map(|adapter| {
                adapter
                    .descriptor()
                    .adopted_files
                    .iter()
                    .map(String::as_str)
            })
            .collect()
    }

    /// The error `target` at `layer.yaml` for a target that has no adapter here (§8 **Targets
    /// without an adapter**).
    #[must_use]
    pub fn refusal(&self, target: &TargetId) -> Diagnostic {
        let available = Stripped::from_untrusted(
            &self
                .targets()
                .map(TargetId::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
        let message = if PLANNED.contains_key(target) {
            Message::new("the '")
                .then(MessagePart::printable(target))
                .then("' adapter is not implemented yet (implemented: ")
                .then(available)
                .then(")")
        } else {
            Message::new("no adapter for '")
                .then(MessagePart::printable(target))
                .then("' (available: ")
                .then(available)
                .then(")")
        };
        Diagnostic::error(
            Code::Target,
            Some(Where::repository_file(Stripped::from_untrusted(
                "layer.yaml",
            ))),
            message,
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod output_checks;
