//! How long a git call may run (spec §6 "Git subprocesses", owner answer 5 of TASK-55.4).

use std::time::Duration;

/// The time a git call may take before its process group is ended. A parameter of the runner, so
/// a test can use a short one; the binary uses [`Deadlines::STANDARD`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadlines {
    /// For `git clone`.
    pub clone: Duration,
    /// For every other call, `git version` included.
    pub other: Duration,
}

impl Deadlines {
    /// 10 minutes for a clone and 2 minutes for any other call.
    pub const STANDARD: Self = Self {
        clone: Duration::from_mins(10),
        other: Duration::from_mins(2),
    };
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Deadlines;

    #[test]
    fn the_standard_deadlines_are_ten_minutes_for_a_clone_and_two_for_other_calls() {
        assert_eq!(Deadlines::STANDARD.clone, Duration::from_mins(10));
        assert_eq!(Deadlines::STANDARD.other, Duration::from_mins(2));
    }
}
