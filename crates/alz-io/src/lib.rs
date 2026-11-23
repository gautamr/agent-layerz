//! Every effect of agent-layerz: file system, processes and environment, each reached through
//! a capability type (decision-23). Only this crate calls `rustix` (decision-27).

mod fs;

pub use fs::{ReadError, ReadOnlyDir};
