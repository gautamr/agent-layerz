//! The account model (spec §7.13): names, the choice a layer declares, and the variables a layer
//! may not set. Folder paths are R6's; nothing here touches the file system.

mod allow;
mod allow_file_problem;
mod allow_hint;
mod allow_state;
mod allow_text;
mod cached;
mod confirmation;
mod declaration;
mod folder_key;
mod folders;
mod name;
mod person;
mod protected;
mod reading;
mod resolution;
mod shower;
mod user_config;
mod which_report;

pub use allow::{AllowDecision, Allowance, Offer, allowance, distinct_names};
pub use allow_file_problem::AllowFileProblem;
pub use allow_hint::{AllowHint, HintFacts, allow_hint};
pub use allow_state::{AllowEntry, AllowFile, AllowState, AllowedSource, AllowedSourceError};
pub use allow_text::{
    AllowCommand, PersonRefusal, Refusal, allowed_lines, already_allowed, not_a_person,
    nothing_to_remove, offer_lines, prompt, remove_offer_lines, removed, unwritable,
};
pub use cached::{
    CachedDeclaration, CachedLayer, DeclaredAccount, Ignored, IndexEntry, Miss, MissReason,
    ReadPlan, decide, has_parent_segment, lock_entry, lock_miss, read_plan,
};
pub use confirmation::{Answer, Confirmation};
pub use declaration::{
    AccountChoice, AccountDeclaration, AccountShapeError, EmptyTargetMap, NonEmptyTargetMap,
};
pub use folder_key::{FolderKey, FolderKeyError, HomeRelative};
pub use folders::{AccountFolders, Relation, lies_within, names_account_folder};
pub use name::{AccountName, AccountNameError};
pub use person::{
    AgentSession, AgentSessionMarker, NotAPerson, decide as decide_person_at_terminal, markers_set,
};
pub use protected::{
    PROTECTED_VARIABLES, codex_pattern_matches, is_protected_name, matched_protected_names,
    protected_variable,
};
pub use reading::{Reading, parse_file, parse_path_file};
pub use resolution::{
    AllowedFact, Applied, BLOCKED, ConfigFact, EnvironmentFact, ExistingFolders, Explicit,
    ExplicitName, Facts, FailsClosed, FileFact, FileOutcome, Fix, Foreign, HomeFact, HookApplies,
    ManifestFact, ManifestProblem, NoRepository, Recorded, Repository, Resolution, SkipReason,
    Skipped, Source, Stack, Step, TargetResolution, VariableFact, Verdict, folder_name, resolve,
    variable,
};
pub use user_config::{FolderRule, UserConfig};
pub use which_report::{WhichReport, which_report};
