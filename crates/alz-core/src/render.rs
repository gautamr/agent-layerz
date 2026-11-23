//! Writing files alz generates: the file tree with its generated header, path globs and protected
//! paths, the lock of a build, the YAML emitter (decision-28) and the stable JSON emitter
//! (decision-33).

pub mod glob;
pub mod header;
pub mod json;
pub mod lock_of;
pub mod protected;
pub mod tree;
pub mod yaml;
