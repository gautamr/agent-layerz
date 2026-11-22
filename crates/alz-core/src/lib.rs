//! The pure domain of agent-layerz.
//!
//! This crate performs no input or output: `clippy.toml` bans the file system, process and
//! environment functions of the standard library (decision-23), and the crate does not depend
//! on `alz-io`.

/// Throwaway clippy failure for the CI check (task-55.1.5); never merged.
#[must_use]
pub fn clippy_fails() -> i32 {
    let x = 1;
    #[allow(clippy::let_and_return)]
    x.clone()
}
