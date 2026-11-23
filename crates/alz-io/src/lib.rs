//! Every effect of agent-layerz: file system, processes and environment, each reached through
//! a capability type (decision-23). Only this crate calls `rustix` (decision-27).

mod env;
mod fs;

pub use env::EnvSnapshot;
pub use fs::{ReadError, ReadOnlyDir};
