//! What `docs/target-facts.md` records about the Claude Code version alz was verified against
//! (spec §8.1 "Verified range"), and the version number as `claude --version` prints it.

use std::fmt;

use regex_lite::Regex;

/// A Claude Code version, `major.minor.patch`. The order is the order of versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// A version from its three numbers.
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// The first `x.y.z` in `text`, which is how `claude --version` prints it (`2.1.311 (Claude
    /// Code)`). `None` when there is none, or a number does not fit in 64 bits.
    #[must_use]
    pub fn find_in(text: &str) -> Option<Self> {
        let pattern = Regex::new(r"(\d+)\.(\d+)\.(\d+)").ok()?;
        let captures = pattern.captures(text)?;
        let number = |index: usize| captures.get(index)?.as_str().parse::<u64>().ok();
        Some(Self::new(number(1)?, number(2)?, number(3)?))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The versions of Claude Code the adapter was checked against: from `min` up to, not including,
/// `below`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedRange {
    min: Version,
    below: Version,
}

/// The range `>=2.1.311 <2.2.0` (spec §8.1).
pub const VERIFIED_RANGE: VerifiedRange = VerifiedRange {
    min: Version::new(2, 1, 311),
    below: Version::new(2, 2, 0),
};

impl VerifiedRange {
    /// Whether `version` was verified.
    #[must_use]
    pub fn contains(&self, version: Version) -> bool {
        self.min <= version && version < self.below
    }
}

impl fmt::Display for VerifiedRange {
    /// `>=2.1.311 <2.2.0`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, ">={} <{}", self.min, self.below)
    }
}

#[cfg(test)]
mod tests {
    use super::{VERIFIED_RANGE, Version};

    #[test]
    fn the_version_is_the_first_triple_in_the_text() {
        assert_eq!(
            Version::find_in("2.1.311 (Claude Code)"),
            Some(Version::new(2, 1, 311))
        );
        assert_eq!(
            Version::find_in("claude v10.0.2 beta 1.2.3"),
            Some(Version::new(10, 0, 2))
        );
    }

    #[test]
    fn text_without_a_version_has_none() {
        assert_eq!(Version::find_in(""), None);
        assert_eq!(Version::find_in("2.1"), None);
        assert_eq!(Version::find_in("99999999999999999999.0.0"), None);
    }

    #[test]
    fn the_range_includes_its_minimum_and_excludes_its_upper_bound() {
        assert!(VERIFIED_RANGE.contains(Version::new(2, 1, 311)));
        assert!(VERIFIED_RANGE.contains(Version::new(2, 1, 999)));
        assert!(!VERIFIED_RANGE.contains(Version::new(2, 1, 310)));
        assert!(!VERIFIED_RANGE.contains(Version::new(2, 2, 0)));
        assert!(!VERIFIED_RANGE.contains(Version::new(10, 0, 0)));
    }

    #[test]
    fn the_range_prints_as_the_spec_writes_it() {
        assert_eq!(VERIFIED_RANGE.to_string(), ">=2.1.311 <2.2.0");
    }
}
