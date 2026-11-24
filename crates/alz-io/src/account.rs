//! The home folder and the two account files, `accounts.yaml` and `allowed.yaml` (spec §7.13
//! "Environment", "User config", "Allow state", "Both files"; TASK-72).
//!
//! [`Home`] finds `$HOME`. [`AccountFiles`] locates each file, applies the trust checks of "Both
//! files", reads it through no link and at most 1 MiB, parses it into `alz-core`'s validated
//! values and checks them again once their paths are real. A file that fails a check is ignored
//! with one warning that holds no text of the file. Account folders are never opened here.

mod files;
mod home;
mod ignore;
mod resolved;
mod state_dir;
mod state_error;
mod state_place;

pub use files::{AccountFiles, FileStatus, Loaded};
pub use home::Home;
pub use ignore::IgnoreReason;
pub use resolved::{ResolvedAllowState, ResolvedFolder, ResolvedUserConfig};
pub use state_dir::{AllowChange, Stage, StateDir, Updated};
pub use state_error::{StateError, StateProblem};
