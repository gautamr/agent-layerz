//! The layer cache as names only (spec §6 "The layer cache"): where the cache root is and what
//! each entry in it is called. Opening, creating and checking the folders is alz-io's.

mod entry_name;
mod location;

pub use entry_name::{CloneKey, DirKey, EntryName, EntryNameError};
pub use location::{CacheLocation, CacheLocationError};
