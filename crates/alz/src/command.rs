//! The commands of the specification. Each one reads what it needs through `alz-io`, decides with
//! `alz-core` and returns the exit code; `main` only dispatches.

pub mod lint;
