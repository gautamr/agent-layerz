//! What `alz init` writes (spec §10, §10.3): the files of a new repository or layer, their text
//! byte for byte, and the checks of the options that decide them. Pure: the files are written by
//! `alz-io`'s repository root.

mod error;
mod files;
mod hub;
mod layer_name;
mod stack_option;

pub use error::{ScaffoldError, StackSourceProblem};
pub use files::{
    MISE_TASKS, RepoOptions, RepoSurroundings, ScaffoldFile, Scaffolded, layer_files,
    manifest_path, mise_config_paths, repo_files, repo_layer_path,
};
pub use hub::Hub;
pub use layer_name::layer_name;
pub use stack_option::stack_entry;
