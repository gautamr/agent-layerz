//! The five steps (spec §7.13 "Resolution"), per target, first match wins:
//! 1. `alz run --account`, 2. `alz use` (`--use`), 3. the user config's longest `folders` entry,
//! 4. the layers' declaration when allowed and readable without a fetch, 5. the default account.
//!
//! A value set outside alz (foreign) replaces steps 3 to 5, never 1 and 2. A named account that
//! cannot apply fails closed: it is never the default account and never a newly declared one.

use std::cell::OnceCell;
use std::collections::BTreeMap;

use super::facts::{
    ExplicitName, Facts, FileOutcome, HomeFact, ManifestFact, NoRepository, Repository, Stack,
};
use super::outcome::{
    Applied, FailsClosed, Foreign, ManifestProblem, Recorded, Resolution, SkipReason, Skipped,
    Source, Step, TargetResolution, Verdict,
};
use crate::account::AccountFamily;
use crate::account::{
    AccountChoice, AccountName, AllowEntry, CachedDeclaration, DeclaredAccount, Ignored,
};
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::merge::Reason;
use crate::path::AbsolutePath;
use crate::untrusted_text::{Quoted, Stripped};

/// Decides the account of each target. Reads nothing: `stack` is asked for the declaration only
/// when a target gets to step 4 in an allowed repository, and then once.
#[must_use]
pub fn resolve(facts: &Facts, stack: &impl Stack) -> Resolution {
    let context = Context {
        facts,
        stack,
        layer: OnceCell::new(),
    };
    let decided: Vec<(AccountFamily, TargetResolution, Option<Diagnostic>)> = AccountFamily::ALL
        .into_iter()
        .map(|target| {
            let (resolution, warning) = context.target(target);
            (target, resolution, warning)
        })
        .collect();
    let layer_warnings = context
        .layer
        .get()
        .map_or_else(Vec::new, |step| step.warnings.clone());
    let warnings = default_name_warning(facts)
        .into_iter()
        .chain(decided.iter().filter_map(|(_, _, warning)| warning.clone()))
        .chain(layer_warnings)
        .fold(Diagnostics::new(), Diagnostics::with);
    Resolution {
        targets: decided
            .into_iter()
            .map(|(target, resolution, _)| (target, resolution))
            .collect(),
        warnings,
        allowed: root_entry(facts).is_some(),
    }
}

fn default_name_warning(facts: &Facts) -> Option<Diagnostic> {
    let name = facts.config.default_name.as_ref()?;
    AccountFamily::ALL
        .into_iter()
        .any(|target| facts.folders.exists(target, name))
        .then(|| {
            Diagnostic::warning(
                Code::AccountDefaultNameFolder,
                Some(Where::given(Stripped::from_untrusted("accounts.yaml"))),
                Message::new("default_name '")
                    .then(MessagePart::printable(name))
                    .then("' is also the name of an account folder (~/.claude-")
                    .then(MessagePart::printable(name))
                    .then(" or ~/.codex-")
                    .then(MessagePart::printable(name))
                    .then("); that folder can never be chosen"),
            )
        })
}

fn root_entry(facts: &Facts) -> Option<&AllowEntry> {
    match &facts.repository {
        Repository::Root { path, .. } => facts
            .allowed
            .entries
            .iter()
            .find(|entry| entry.root() == path),
        Repository::None(_) => None,
    }
}

struct Context<'a, S> {
    facts: &'a Facts,
    stack: &'a S,
    layer: OnceCell<LayerStep>,
}

/// What step 4 found, once, for every target.
struct LayerStep {
    outcome: Layer,
    warnings: Vec<Diagnostic>,
}

enum Layer {
    /// The step names nothing and nothing fails.
    Skipped(SkipReason),
    /// The allowed declaration applies.
    Declared(BTreeMap<AccountFamily, AccountChoice>),
    /// The targets that fail closed; a target left out is skipped.
    Fails(BTreeMap<AccountFamily, FailsClosed>),
}

impl<S: Stack> Context<'_, S> {
    fn target(&self, target: AccountFamily) -> (TargetResolution, Option<Diagnostic>) {
        let explicit = self
            .facts
            .explicit
            .run
            .iter()
            .find(|(chosen, _)| *chosen == target)
            .map(|(_, name)| (Source::Run, name))
            .or_else(|| {
                self.facts
                    .explicit
                    .used
                    .iter()
                    .find(|(chosen, _)| *chosen == target)
                    .map(|(_, name)| (Source::Use, name))
            });
        if let Some((source, name)) = explicit {
            return (self.explicit(target, source, name), None);
        }
        let skipped = vec![Skipped {
            step: Step::Use,
            reason: SkipReason::NoUseChoice,
        }];
        if let Some(value) = self.facts.environment.variable(target).foreign() {
            return (
                TargetResolution {
                    source: Source::Environment,
                    skipped,
                    verdict: Verdict::Applied(Applied::Kept(Foreign::new(value.to_owned()))),
                },
                Some(self.foreign_warning(target)),
            );
        }
        (self.steps_three_to_five(target, skipped), None)
    }

    fn explicit(
        &self,
        target: AccountFamily,
        source: Source,
        name: &ExplicitName,
    ) -> TargetResolution {
        match name {
            ExplicitName::Valid(choice) => self.named(target, choice, source, Vec::new()),
            ExplicitName::NotAName => TargetResolution {
                source,
                skipped: Vec::new(),
                verdict: Verdict::FailsClosed(FailsClosed::InvalidName),
            },
        }
    }

    fn foreign_warning(&self, target: AccountFamily) -> Diagnostic {
        let message = Message::new(super::outcome::variable(target)).then(" was set outside alz");
        Diagnostic::warning(
            Code::AccountForeign,
            None,
            if self.facts.environment.manager_active {
                message.then("; mise or direnv is active here")
            } else {
                message
            }
            .then("; alz keeps it"),
        )
    }

    /// The account of a name that was chosen: the default account, the folder of a named
    /// account that exists, or a failure.
    fn named(
        &self,
        target: AccountFamily,
        choice: &AccountChoice,
        source: Source,
        skipped: Vec<Skipped>,
    ) -> TargetResolution {
        let verdict = match choice {
            AccountChoice::Default => Verdict::Applied(Applied::Default),
            AccountChoice::Named(name) if self.is_default_name(name) => {
                Verdict::Applied(Applied::Default)
            }
            AccountChoice::Named(name) => match &self.facts.home {
                HomeFact::NotAbsolute => Verdict::FailsClosed(FailsClosed::HomeNotAbsolute),
                HomeFact::Absolute { .. } if self.facts.folders.exists(target, name) => {
                    Verdict::Applied(Applied::Folder(name.clone()))
                }
                HomeFact::Absolute { .. } => Verdict::FailsClosed(FailsClosed::MissingFolder {
                    name: name.clone(),
                    existing: self.facts.folders.names(target).cloned().collect(),
                }),
            },
        };
        TargetResolution {
            source,
            skipped,
            verdict,
        }
    }

    fn is_default_name(&self, name: &AccountName) -> bool {
        self.facts.config.default_name.as_ref() == Some(name)
    }

    fn steps_three_to_five(
        &self,
        target: AccountFamily,
        skipped: Vec<Skipped>,
    ) -> TargetResolution {
        let skip = |mut skipped: Vec<Skipped>, step, reason| {
            skipped.push(Skipped { step, reason });
            skipped
        };
        let matched = self.matched_folder();
        let from_config = matched
            .and_then(|declaration| declaration.choice_for(target))
            .cloned();
        if let Some(choice) = from_config {
            return self.named(target, &choice, Source::UserConfig, skipped);
        }
        let skipped = skip(
            skipped,
            Step::UserConfig,
            self.config_skip(matched.is_some()),
        );
        let layer = self.layer();
        let skipped = match &layer.outcome {
            Layer::Declared(choices) => match choices.get(&target) {
                Some(choice) => return self.named(target, choice, Source::Layer, skipped),
                None => skip(skipped, Step::Layer, SkipReason::LayersNameNoTarget),
            },
            Layer::Skipped(reason) => skip(skipped, Step::Layer, *reason),
            Layer::Fails(failing) => match failing.get(&target) {
                Some(failure) => {
                    return TargetResolution {
                        source: Source::Layer,
                        skipped,
                        verdict: Verdict::FailsClosed(failure.clone()),
                    };
                }
                None => skip(skipped, Step::Layer, SkipReason::LayersNameNoTarget),
            },
        };
        TargetResolution {
            source: Source::Default,
            skipped,
            verdict: Verdict::Applied(Applied::Default),
        }
    }

    /// Step 3: the entry with the longest real path that is the working directory or one of its
    /// parents. A working directory inside an account folder or that does not resolve matches
    /// nothing. A shorter entry is never tried when the longest names no account.
    fn matched_folder(&self) -> Option<&crate::account::AccountDeclaration> {
        let cwd = self.facts.known_directory.as_ref()?;
        match &self.facts.repository {
            Repository::None(NoRepository::AccountFolder | NoRepository::Unresolvable) => None,
            Repository::None(NoRepository::NotFound | NoRepository::ConfiguredFolder)
            | Repository::Root { .. } => self
                .facts
                .config
                .folders
                .iter()
                .filter(|(real, _)| is_parent_or_same(real, cwd))
                .max_by_key(|(real, _)| real.components().count())
                .map(|(_, declaration)| declaration),
        }
    }

    fn config_skip(&self, matched: bool) -> SkipReason {
        match &self.facts.repository {
            Repository::None(NoRepository::AccountFolder) => SkipReason::CwdInAccountFolder,
            Repository::None(NoRepository::Unresolvable) => SkipReason::CwdUnresolvable,
            Repository::None(NoRepository::NotFound | NoRepository::ConfiguredFolder)
            | Repository::Root { .. } => match &self.facts.config.file.outcome {
                FileOutcome::Ignored(_) => SkipReason::UserConfigIgnored,
                FileOutcome::NotFound => SkipReason::NoUserConfig,
                FileOutcome::Read if matched => SkipReason::EntryNamesNoTarget,
                FileOutcome::Read => SkipReason::NoFoldersMatch,
            },
        }
    }

    fn layer(&self) -> &LayerStep {
        self.layer.get_or_init(|| self.layer_step())
    }

    /// Step 4, once: the stack is read at most one time, and only for an allowed root.
    fn layer_step(&self) -> LayerStep {
        let facts = self.facts;
        let skipped = |reason| LayerStep {
            outcome: Layer::Skipped(reason),
            warnings: Vec::new(),
        };
        let allowed_file = match &facts.allowed.file.outcome {
            FileOutcome::NotFound => Some(SkipReason::AllowedMissing),
            FileOutcome::Ignored(_) => Some(SkipReason::AllowedIgnored),
            FileOutcome::Read => None,
        };
        match &facts.repository {
            Repository::None(NoRepository::ConfiguredFolder) => {
                self.configured_folder(allowed_file)
            }
            Repository::None(NoRepository::NotFound) => skipped(SkipReason::NoRepository),
            Repository::None(NoRepository::AccountFolder) => {
                skipped(SkipReason::CwdInAccountFolder)
            }
            Repository::None(NoRepository::Unresolvable) => skipped(SkipReason::CwdUnresolvable),
            Repository::Root { manifest, .. } => match (allowed_file, root_entry(facts)) {
                (Some(reason), _) => skipped(reason),
                (None, None) => skipped(SkipReason::NotAllowed),
                (None, Some(entry)) => self.allowed_root(entry, *manifest),
            },
        }
    }

    /// A working directory inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names: nothing in
    /// it is opened, so the working directory is known only as that folder. An allowed entry whose
    /// root is the folder, a parent of it or lies inside it fails closed on the blocked folder:
    /// the working directory may be that repository, and what is below the folder is not looked
    /// at to tell.
    fn configured_folder(&self, allowed_file: Option<SkipReason>) -> LayerStep {
        let facts = self.facts;
        let skipped = |reason| LayerStep {
            outcome: Layer::Skipped(reason),
            warnings: Vec::new(),
        };
        let Some(known) = facts.known_directory.as_ref() else {
            return skipped(SkipReason::CwdInConfiguredFolder);
        };
        if let Some(reason) = allowed_file {
            return skipped(reason);
        }
        let covering: Vec<&AllowEntry> = facts
            .allowed
            .entries
            .iter()
            .filter(|entry| {
                is_parent_or_same(entry.root(), known) || is_parent_or_same(known, entry.root())
            })
            .collect();
        if covering.is_empty() {
            return skipped(SkipReason::CwdInConfiguredFolder);
        }
        LayerStep {
            outcome: Layer::Fails(
                AccountFamily::ALL
                    .into_iter()
                    .filter(|target| {
                        covering
                            .iter()
                            .any(|entry| entry.account().get(*target).is_some())
                    })
                    .map(|target| (target, FailsClosed::ConfiguredFolder))
                    .collect(),
            ),
            warnings: Vec::new(),
        }
    }

    fn allowed_root(&self, entry: &AllowEntry, manifest: ManifestFact) -> LayerStep {
        let allowed_targets = || {
            AccountFamily::ALL
                .into_iter()
                .filter(|target| entry.account().get(*target).is_some())
        };
        let failing = |failure: FailsClosed| LayerStep {
            outcome: Layer::Fails(
                allowed_targets()
                    .map(|target| (target, failure.clone()))
                    .collect(),
            ),
            warnings: Vec::new(),
        };
        match manifest {
            ManifestFact::Unreadable => {
                return failing(FailsClosed::Manifest(ManifestProblem::Unreadable));
            }
            ManifestFact::Invalid => {
                return failing(FailsClosed::Manifest(ManifestProblem::Invalid));
            }
            ManifestFact::Read => {}
        }
        match self.stack.declaration() {
            CachedDeclaration::Miss(miss) => {
                if self.stack.contains(entry.source()) {
                    LayerStep {
                        outcome: Layer::Fails(
                            allowed_targets()
                                .map(|target| {
                                    (
                                        target,
                                        FailsClosed::CacheMiss {
                                            miss: miss.reason(),
                                            recorded: self.recorded(target, entry),
                                        },
                                    )
                                })
                                .collect(),
                        ),
                        warnings: Vec::new(),
                    }
                } else {
                    failing(FailsClosed::SourceGone)
                }
            }
            CachedDeclaration::Conflict { ignored, .. } => LayerStep {
                warnings: ignored_warnings(&ignored),
                ..failing(FailsClosed::Conflict)
            },
            CachedDeclaration::None { ignored } => LayerStep {
                warnings: ignored_warnings(&ignored),
                ..failing(FailsClosed::Changed {
                    still_declared: false,
                })
            },
            CachedDeclaration::Declared {
                declaration,
                ignored,
            } => {
                let step = declared_step(entry, &declaration);
                LayerStep {
                    warnings: ignored_warnings(&ignored)
                        .into_iter()
                        .chain(step.warnings)
                        .collect(),
                    outcome: step.outcome,
                }
            }
        }
    }

    fn recorded(&self, target: AccountFamily, entry: &AllowEntry) -> Recorded {
        let choice = entry
            .account()
            .get(target)
            .map_or(AccountChoice::Default, choice_of);
        match choice {
            AccountChoice::Default => Recorded::Default,
            AccountChoice::Named(name) if self.is_default_name(&name) => Recorded::Default,
            AccountChoice::Named(name) => Recorded::Folder {
                exists: self.facts.folders.exists(target, &name),
                name,
            },
        }
    }
}

/// An allowed name as a choice: `default` is the default account.
fn choice_of(name: &AccountName) -> AccountChoice {
    if name.as_str() == "default" {
        AccountChoice::Default
    } else {
        AccountChoice::Named(name.clone())
    }
}

fn is_parent_or_same(parent: &AbsolutePath, directory: &AbsolutePath) -> bool {
    let wanted: Vec<&str> = parent.components().collect();
    let found: Vec<&str> = directory.components().collect();
    found.starts_with(&wanted)
}

fn choice_text(choice: &AccountChoice) -> Message {
    match choice {
        AccountChoice::Default => Message::new("default"),
        AccountChoice::Named(name) => Message::new(MessagePart::printable(name)),
    }
}

fn map_text(map: &BTreeMap<AccountFamily, AccountChoice>) -> Message {
    map.iter()
        .enumerate()
        .fold(Message::new(""), |message, (index, (target, choice))| {
            let separated = if index == 0 {
                message
            } else {
                message.then(", ")
            };
            separated
                .then(MessagePart::printable(target))
                .then("=")
                .and(choice_text(choice))
        })
}

fn changed_warning(
    entry: &AllowEntry,
    recorded: &BTreeMap<AccountFamily, AccountChoice>,
    declared: &BTreeMap<AccountFamily, AccountChoice>,
    declaration: &DeclaredAccount,
) -> Diagnostic {
    Diagnostic::warning(
        Code::AccountChanged,
        None,
        Message::new("the account declaration changed since it was allowed: was ")
            .and(map_text(recorded))
            .then(" from ")
            .then(Quoted::from_untrusted(entry.source().as_str()))
            .then(", now ")
            .and(map_text(declared))
            .then(" from ")
            .then(Quoted::from_untrusted(&declaration.recorded_source)),
    )
}

pub(crate) fn ignored_warnings(ignored: &[Ignored]) -> Vec<Diagnostic> {
    ignored
        .iter()
        .map(|item| {
            Diagnostic::warning(
                Code::AccountDeclarationIgnored,
                None,
                Message::new("the layer ")
                    .then(Quoted::from_untrusted(&item.layer.to_string()))
                    .then(" (")
                    .then(Quoted::from_untrusted(&item.source.to_string()))
                    .then(") declares an account but may not, so it is ignored: ")
                    .and(reasons_text(&item.reasons)),
            )
        })
        .collect()
}

fn reasons_text(reasons: &[Reason]) -> Message {
    reasons
        .iter()
        .enumerate()
        .fold(Message::new(""), |message, (index, reason)| {
            let separated = if index == 0 {
                message
            } else {
                message.then("; ")
            };
            match reason {
                Reason::InsideRepository => separated.then("it is inside the repository"),
                Reason::KindNotClient(kind) => separated
                    .then("its kind is '")
                    .then(Stripped::from_untrusted(kind.as_str()))
                    .then("', not 'client'"),
                Reason::Public => separated.then("it is public"),
            }
        })
}

/// Step 4 for a declaration: it applies when its map and its source are the recorded ones,
/// and otherwise every target either names fails closed.
fn declared_step(entry: &AllowEntry, declaration: &DeclaredAccount) -> LayerStep {
    let declared: BTreeMap<AccountFamily, AccountChoice> = AccountFamily::ALL
        .into_iter()
        .filter_map(|target| {
            declaration
                .map
                .choice_for(target)
                .map(|choice| (target, choice.clone()))
        })
        .collect();
    let recorded: BTreeMap<AccountFamily, AccountChoice> = AccountFamily::ALL
        .into_iter()
        .filter_map(|target| {
            entry
                .account()
                .get(target)
                .map(|name| (target, choice_of(name)))
        })
        .collect();
    if declared == recorded && declaration.recorded_source == entry.source().as_str() {
        return LayerStep {
            outcome: Layer::Declared(declared),
            warnings: Vec::new(),
        };
    }
    let affected = AccountFamily::ALL
        .into_iter()
        .filter(|target| declared.contains_key(target) || recorded.contains_key(target));
    LayerStep {
        outcome: Layer::Fails(
            affected
                .map(|target| {
                    (
                        target,
                        FailsClosed::Changed {
                            still_declared: true,
                        },
                    )
                })
                .collect(),
        ),
        warnings: vec![changed_warning(entry, &recorded, &declared, declaration)],
    }
}
