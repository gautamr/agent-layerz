//! What resolution decides, per target (spec §7.13 "Resolution", "A named account that cannot
//! apply fails closed"; doc-6 §3.2). A target is [`Applied`] or [`FailsClosed`], never both, and
//! each `FailsClosed` says what the hook applies and which command fixes it.

use std::collections::BTreeMap;
use std::fmt;

use crate::account::AccountFamily;
use crate::account::{AccountName, MissReason};
use crate::diagnostic::Diagnostics;

/// The step that decided a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Step 1: `alz run --account`.
    Run,
    /// Step 2: `alz use`, passed as `--use`.
    Use,
    /// A value set outside alz, which takes the place of steps 3 to 5.
    Environment,
    /// Step 3: `folders` of `accounts.yaml`.
    UserConfig,
    /// Step 4: the layers' declaration, allowed.
    Layer,
    /// Step 5.
    Default,
}

impl Source {
    /// The word of the report.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Use => "use",
            Self::Environment => "environment",
            Self::UserConfig => "user config",
            Self::Layer => "layer",
            Self::Default => "default",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A step that was consulted before the one that decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Step 2.
    Use,
    /// Step 3.
    UserConfig,
    /// Step 4.
    Layer,
}

impl Step {
    /// The word of the report.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Use => "use",
            Self::UserConfig => "user config",
            Self::Layer => "layer",
        }
    }
}

/// Why a step named no account for a target. It is not a failure: the next step is tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// No `--use` for the target.
    NoUseChoice,
    /// No `accounts.yaml`.
    NoUserConfig,
    /// `accounts.yaml` is ignored.
    UserConfigIgnored,
    /// No `folders` entry holds the working directory.
    NoFoldersMatch,
    /// The entry names no account for the target.
    EntryNamesNoTarget,
    /// No repository.
    NoRepository,
    /// The working directory is inside an account folder.
    CwdInAccountFolder,
    /// The working directory does not resolve.
    CwdUnresolvable,
    /// The working directory is inside a configured folder and no allowed entry covers it.
    CwdInConfiguredFolder,
    /// No `allowed.yaml`.
    AllowedMissing,
    /// `allowed.yaml` is ignored.
    AllowedIgnored,
    /// The root has no entry.
    NotAllowed,
    /// The layers (or the allowed entry) name no account for the target.
    LayersNameNoTarget,
}

/// A step skipped before the one that decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skipped {
    /// The step.
    pub step: Step,
    /// Why.
    pub reason: SkipReason,
}

/// A variable's value that was set outside alz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foreign(String);

impl Foreign {
    pub(super) const fn new(value: String) -> Self {
        Self(value)
    }

    /// The value as it was set; untrusted text, so a report shows it stripped.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A target that resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// The variable is set to the folder of this account.
    Folder(AccountName),
    /// The variable is unset: the default account (`default`, or the configured name).
    Default,
    /// The variable is left as it is: set outside alz.
    Kept(Foreign),
}

/// The account a cache miss records for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorded {
    /// The default account.
    Default,
    /// A named account, and whether its folder exists.
    Folder {
        /// The recorded name.
        name: AccountName,
        /// Whether `~/.claude-<name>` or `~/.codex-<name>` exists.
        exists: bool,
    },
}

/// Why the manifest of an allowed root cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestProblem {
    /// `layer.yaml` cannot be read.
    Unreadable,
    /// `layer.yaml` is not valid.
    Invalid,
}

/// A target that cannot apply. alz never uses the default account for it and never a newly
/// declared one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailsClosed {
    /// An explicit name that is not an account name.
    InvalidName,
    /// `HOME` is not an absolute path, so no folder can be named.
    HomeNotAbsolute,
    /// The named account's folder does not exist.
    MissingFolder {
        /// The name.
        name: AccountName,
        /// The folders that exist for the target.
        existing: Vec<AccountName>,
    },
    /// The layers cannot be verified; the recorded account stays.
    CacheMiss {
        /// What could not be verified.
        miss: MissReason,
        /// What the allowed entry records.
        recorded: Recorded,
    },
    /// The declaration changed since it was allowed, or is gone.
    Changed {
        /// Whether the layers still declare (`alz account allow`), or no longer
        /// (`alz account allow --remove`).
        still_declared: bool,
    },
    /// The recorded source is no longer in the stack.
    SourceGone,
    /// Several client layers declare.
    Conflict,
    /// The manifest of the allowed root cannot be used.
    Manifest(ManifestProblem),
    /// An allowed repository lies in a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names.
    ConfiguredFolder,
}

/// What the shell hook applies for a target that fails closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookApplies {
    /// Nothing: no folder can be named.
    NoFolder,
    /// The folder of the account that does not exist.
    MissingFolder(AccountName),
    /// The recorded account (the variable unset for the default).
    Recorded(Recorded),
    /// `~/.claude-alz-blocked` or `~/.codex-alz-blocked`.
    Blocked,
}

/// The command that fixes a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    /// Use a valid name.
    UseAValidName,
    /// Set `HOME`.
    SetHome,
    /// `alz account init <name>`.
    AccountInit(AccountName),
    /// The name is reserved.
    ReservedName,
    /// `alz build`.
    Build,
    /// `alz account allow`.
    Allow,
    /// `alz account allow --remove`.
    AllowRemove,
    /// Unset the variables or point them outside the repository.
    UnsetConfiguredFolders,
    /// Remove all but one declaration, then `alz build`.
    RemoveDeclarations,
    /// Repair `<root>/layer.yaml`.
    RepairManifest,
}

impl FailsClosed {
    /// What the hook applies.
    #[must_use]
    pub fn hook(&self) -> HookApplies {
        match self {
            Self::InvalidName | Self::HomeNotAbsolute => HookApplies::NoFolder,
            Self::MissingFolder { name, .. } => HookApplies::MissingFolder(name.clone()),
            Self::CacheMiss { recorded, .. } => HookApplies::Recorded(recorded.clone()),
            Self::Changed { .. }
            | Self::SourceGone
            | Self::Conflict
            | Self::Manifest(_)
            | Self::ConfiguredFolder => HookApplies::Blocked,
        }
    }

    /// The command that fixes it.
    #[must_use]
    pub fn fix(&self) -> Fix {
        match self {
            Self::InvalidName => Fix::UseAValidName,
            Self::HomeNotAbsolute => Fix::SetHome,
            Self::MissingFolder { name, .. } if name.as_str() == super::facts::BLOCKED => {
                Fix::ReservedName
            }
            Self::MissingFolder { name, .. } => Fix::AccountInit(name.clone()),
            Self::CacheMiss { .. } => Fix::Build,
            Self::Changed {
                still_declared: false,
            } => Fix::AllowRemove,
            Self::Changed {
                still_declared: true,
            }
            | Self::SourceGone => Fix::Allow,
            Self::Conflict => Fix::RemoveDeclarations,
            Self::Manifest(_) => Fix::RepairManifest,
            Self::ConfiguredFolder => Fix::UnsetConfiguredFolders,
        }
    }
}

/// A target's account: applied, or failing closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It resolves.
    Applied(Applied),
    /// It cannot apply.
    FailsClosed(FailsClosed),
}

/// The decision for one target, with the steps that were skipped on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetResolution {
    /// The step that decided.
    pub source: Source,
    /// The steps consulted before it that named no account.
    pub skipped: Vec<Skipped>,
    /// The decision.
    pub verdict: Verdict,
}

/// The decision for every target, and the warnings resolution itself raises (a value set outside
/// alz, a declaration that may not declare, a changed declaration, a default name that is also a
/// folder), each once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub(super) targets: BTreeMap<AccountFamily, TargetResolution>,
    pub(super) warnings: Diagnostics,
    pub(super) allowed: bool,
}

impl Resolution {
    /// The decision for `target`.
    #[must_use]
    pub fn target(&self, target: AccountFamily) -> Option<&TargetResolution> {
        self.targets.get(&target)
    }

    /// Each target and its decision, in target order.
    pub fn targets(&self) -> impl Iterator<Item = (AccountFamily, &TargetResolution)> + '_ {
        self.targets.iter().map(|(target, found)| (*target, found))
    }

    /// The warnings of the resolution.
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
    }

    /// Whether an `allowed.yaml` entry matches the repository root.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Whether every target resolves.
    #[must_use]
    pub fn all_resolve(&self) -> bool {
        self.targets
            .values()
            .all(|found| matches!(found.verdict, Verdict::Applied(_)))
    }
}

/// The variable a target's account is set through.
#[must_use]
pub const fn variable(target: AccountFamily) -> &'static str {
    match target {
        AccountFamily::Claude => "CLAUDE_CONFIG_DIR",
        AccountFamily::Codex => "CODEX_HOME",
    }
}

/// The name of the account folder below `$HOME`, `.claude-<name>` or `.codex-<name>`.
#[must_use]
pub fn folder_name(target: AccountFamily, name: &impl fmt::Display) -> String {
    match target {
        AccountFamily::Claude => format!(".claude-{name}"),
        AccountFamily::Codex => format!(".codex-{name}"),
    }
}
