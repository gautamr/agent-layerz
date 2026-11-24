//! The plan of the repository writer (spec §4.3 "Which files are alz's own", "Links",
//! "Writing"): pure, from the files a build makes, what the writer observed on disk and the
//! previous lock. The effects are `alz-io`'s `RepoRoot`.

mod disk;
mod plan;

pub use disk::{Disk, MAX_COMPARED_BYTES, Observed};
pub use plan::{
    Change, ClearPlan, Collision, CollisionReason, WritePlan, gitignore_path, lock_path,
    plan_writes,
};

pub(crate) use plan::owns;

#[cfg(test)]
mod tests;
