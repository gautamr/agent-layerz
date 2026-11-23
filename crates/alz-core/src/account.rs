//! The account model (spec §7.13): names, the choice a layer declares, and the variables a layer
//! may not set. Folder paths are R6's; nothing here touches the file system.

mod allow_state;
mod declaration;
mod folder_key;
mod folders;
mod name;
mod protected;
mod reading;
mod user_config;

pub use allow_state::{AllowEntry, AllowFile, AllowState, AllowedSource, AllowedSourceError};
pub use declaration::{
    AccountChoice, AccountDeclaration, AccountShapeError, EmptyTargetMap, NonEmptyTargetMap,
};
pub use folder_key::{FolderKey, FolderKeyError, HomeRelative};
pub use folders::{AccountFolders, Relation};
pub use name::{AccountName, AccountNameError};
pub use protected::{
    PROTECTED_VARIABLES, codex_pattern_matches, is_protected_name, matched_protected_names,
};
pub use reading::Reading;
pub use user_config::{FolderRule, UserConfig};
