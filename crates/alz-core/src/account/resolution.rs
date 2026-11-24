//! Account resolution as a pure decision (spec §7.13 "Resolution", "A named account that cannot
//! apply fails closed", "Values set outside alz"; doc-6 §3.2; TASK-55.7.9).
//!
//! [`resolve`] is given [`Facts`], values that effects found and that are already parsed, and a
//! [`Stack`] that reads the layers only when step 4 needs them. It returns, per target, the step
//! that decided, the steps skipped before it and a [`Verdict`]: [`Applied`] with what the hook
//! applies, or [`FailsClosed`] with the reason, what the hook applies instead and the fix. It
//! reads no file, no variable and no clock.

mod facts;
mod outcome;
mod steps;
#[cfg(test)]
mod tests;

pub use facts::{
    AllowedFact, BLOCKED, ConfigFact, EnvironmentFact, ExistingFolders, Explicit, ExplicitName,
    Facts, FileFact, FileOutcome, HomeFact, ManifestFact, NoRepository, Repository, Stack,
    VariableFact,
};
pub use outcome::{
    Applied, FailsClosed, Fix, Foreign, HookApplies, ManifestProblem, Recorded, Resolution,
    SkipReason, Skipped, Source, Step, TargetResolution, Verdict, folder_name, variable,
};
pub(crate) use steps::ignored_warnings;
pub use steps::resolve;
