//! The home folder and the two account files, `accounts.yaml` and `allowed.yaml` (spec §7.13
//! "Environment", "User config", "Allow state", "Both files"; TASK-72).
//!
//! [`Home`] finds `$HOME`. [`AccountFiles`] locates each file, applies the trust checks of "Both
//! files", reads it through no link and at most 1 MiB, parses it into `alz-core`'s validated
//! values and checks them again once their paths are real. A file that fails a check is ignored
//! with one warning that holds no text of the file. [`Place`] says where the working directory is,
//! once; [`find_root`] finds the repository root from it; [`AccountFolderNames`] lists the account
//! folders that exist. Account folders are never opened here.

mod cached_layers;
mod cached_reader;
mod files;
mod home;
mod ignore;
mod listing;
mod place;
mod resolved;
mod root_walk;
mod state_dir;
mod state_error;
mod state_place;

pub use cached_layers::CachedLayers;
pub use cached_reader::CachedDeclarationReader;
pub use files::{AccountFiles, FileStatus, Loaded};
pub use home::Home;
pub use ignore::IgnoreReason;
pub use listing::AccountFolderNames;
pub use place::{OnDisk, Place, PlaceLookup};
pub use resolved::{ResolvedAllowState, ResolvedFolder, ResolvedUserConfig};
pub use root_walk::{ManifestState, NoRoot, Repository, Root, RootSearch, find_root};
pub use state_dir::{AllowChange, Stage, StateDir, Updated};
pub use state_error::{StateError, allow_file_problem};
