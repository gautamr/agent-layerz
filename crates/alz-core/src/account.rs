//! The account model (spec §7.13): names, the choice a layer declares, and the variables a layer
//! may not set. Folder paths are R6's; nothing here touches the file system.

mod declaration;
mod name;
mod protected;

pub use declaration::{
    AccountChoice, AccountDeclaration, AccountShapeError, EmptyTargetMap, NonEmptyTargetMap,
};
pub use name::{AccountName, AccountNameError};
pub use protected::{
    PROTECTED_VARIABLES, codex_pattern_matches, is_protected_name, matched_protected_names,
};
