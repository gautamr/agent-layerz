//! What account resolution is told (spec §7.13 "Resolution"). Every value here was found by an
//! effect somewhere else and is already parsed into a validated type, so the decision itself reads
//! no file, no environment variable and no clock. The one thing it may ask for later, because
//! reading it is the costly step, is the layers' declaration ([`Stack`]).

use std::collections::BTreeSet;

use crate::account::{
    AccountChoice, AccountDeclaration, AccountName, AllowEntry, AllowedSource, CachedDeclaration,
};
use crate::diagnostic::Message;
use crate::path::AbsolutePath;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

/// `$HOME`: an absolute path, or not, in which case no folder is named and neither file is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeFact {
    /// `$HOME` as given, and its real path when it differs.
    Absolute {
        /// The value of `HOME`, from which the folders are built.
        given: AbsolutePath,
        /// The real path of it, shown as `~` too.
        real: Option<AbsolutePath>,
    },
    /// Empty, relative or without a user database entry.
    NotAbsolute,
}

/// What became of `accounts.yaml` or `allowed.yaml`, with the path as a report shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFact {
    /// The path the file is looked for at, `~` for `$HOME`.
    pub shown: Stripped,
    /// What became of it.
    pub outcome: FileOutcome,
}

/// What became of an account file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOutcome {
    /// Not there: the step is skipped.
    NotFound,
    /// Refused or not valid YAML; the reason as the report words it.
    Ignored(Message),
    /// Read, in part at least.
    Read,
}

impl FileOutcome {
    /// Whether the file was read.
    #[must_use]
    pub const fn is_read(&self) -> bool {
        matches!(self, Self::Read)
    }
}

/// `accounts.yaml` as resolution needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFact {
    /// The file.
    pub file: FileFact,
    /// What the default account is called here.
    pub default_name: Option<AccountName>,
    /// The `folders` entries that survived, by real path.
    pub folders: Vec<(AbsolutePath, AccountDeclaration)>,
}

/// `allowed.yaml` as resolution needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedFact {
    /// The file.
    pub file: FileFact,
    /// The valid entries whose root resolves, by real root.
    pub entries: Vec<AllowEntry>,
    /// The valid entries whose root does not exist here.
    pub unresolved: Vec<AbsolutePath>,
}

/// What the root's `layer.yaml` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFact {
    /// A stack manifest that passes the schema.
    Read,
    /// Cannot be read.
    Unreadable,
    /// Read, but not a valid stack.
    Invalid,
}

/// Why there is no repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoRepository {
    /// No manifest on the way, or `$HOME` reached first.
    NotFound,
    /// The working directory is inside an account folder.
    AccountFolder,
    /// The working directory is inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names.
    ConfiguredFolder,
    /// The working directory does not resolve.
    Unresolvable,
}

/// The repository of the working directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repository {
    /// A root and its manifest.
    Root {
        /// The real path of the root.
        path: AbsolutePath,
        /// What its manifest is.
        manifest: ManifestFact,
    },
    /// None.
    None(NoRepository),
}

/// An account variable and its marker, by value (spec §7.13 "Values set outside alz").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableFact {
    /// Not set; a marker without it is stale and never read.
    Unset,
    /// Set, possibly to the empty text.
    Set {
        /// The value of the variable.
        value: String,
        /// The value of its `ALZ_SET_*` marker, when that is set.
        marker: Option<String>,
    },
}

impl VariableFact {
    /// The value when it was set outside alz: a set value, an empty one included, that differs from
    /// its marker or has none.
    #[must_use]
    pub fn foreign(&self) -> Option<&str> {
        match self {
            Self::Unset => None,
            Self::Set { value, marker } => {
                (marker.as_deref() != Some(value.as_str())).then_some(value.as_str())
            }
        }
    }
}

/// The account variables of the environment, and whether mise or direnv is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentFact {
    /// `CLAUDE_CONFIG_DIR`.
    pub claude: VariableFact,
    /// `CODEX_HOME`.
    pub codex: VariableFact,
    /// `__MISE_DIFF` or `DIRENV_DIFF` exists.
    pub manager_active: bool,
}

impl EnvironmentFact {
    /// The variable of `target`.
    #[must_use]
    pub const fn variable(&self, target: TargetName) -> &VariableFact {
        match target {
            TargetName::Claude => &self.claude,
            TargetName::Codex => &self.codex,
        }
    }
}

/// A name from `alz run --account` or `--use`: valid, or not an account name (which fails closed
/// and is never repeated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplicitName {
    /// An account name, or `default`.
    Valid(AccountChoice),
    /// Anything else.
    NotAName,
}

/// Steps 1 and 2: what `alz run --account` and `--use` choose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Explicit {
    /// Step 1: one target, for this run only.
    pub run: Option<(TargetName, ExplicitName)>,
    /// Step 2: the choice the shell function passes, per target.
    pub used: Vec<(TargetName, ExplicitName)>,
}

/// The account folders that exist, from one listing of `$HOME`. The reserved name `alz-blocked`
/// never exists, whatever the listing holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExistingFolders {
    claude: BTreeSet<AccountName>,
    codex: BTreeSet<AccountName>,
}

/// The reserved name of the blocked folders.
pub const BLOCKED: &str = "alz-blocked";

impl ExistingFolders {
    /// The folders of each target, without the blocked one.
    #[must_use]
    pub fn new(
        claude: impl IntoIterator<Item = AccountName>,
        codex: impl IntoIterator<Item = AccountName>,
    ) -> Self {
        let kept = |names: BTreeSet<AccountName>| {
            names
                .into_iter()
                .filter(|name| name.as_str() != BLOCKED)
                .collect()
        };
        Self {
            claude: kept(claude.into_iter().collect()),
            codex: kept(codex.into_iter().collect()),
        }
    }

    /// The names of `target`, in name order.
    pub fn names(&self, target: TargetName) -> impl Iterator<Item = &AccountName> + '_ {
        self.of(target).iter()
    }

    /// Whether the folder of `name` exists for `target`.
    #[must_use]
    pub fn exists(&self, target: TargetName, name: &AccountName) -> bool {
        self.of(target).contains(name)
    }

    const fn of(&self, target: TargetName) -> &BTreeSet<AccountName> {
        match target {
            TargetName::Claude => &self.claude,
            TargetName::Codex => &self.codex,
        }
    }
}

/// The layers, which are read only when step 4 gets to them and only for an allowed repository.
pub trait Stack {
    /// What the layers declare, read from the cache without a fetch. Called at most once.
    fn declaration(&self) -> CachedDeclaration;

    /// Whether the source recorded in `allowed.yaml` is still in the stack.
    fn contains(&self, recorded: &AllowedSource) -> bool;
}

/// Everything resolution is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// `$HOME`.
    pub home: HomeFact,
    /// The account variables.
    pub environment: EnvironmentFact,
    /// Steps 1 and 2.
    pub explicit: Explicit,
    /// `accounts.yaml`.
    pub config: ConfigFact,
    /// `allowed.yaml`.
    pub allowed: AllowedFact,
    /// The repository.
    pub repository: Repository,
    /// The real working directory, or the folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names that
    /// holds it; `None` when it does not resolve, is inside an account folder or is `$HOME` or above.
    pub known_directory: Option<AbsolutePath>,
    /// The account folders that exist.
    pub folders: ExistingFolders,
}
