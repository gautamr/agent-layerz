//! The version of the model (spec §8.0 **Model.**): `major.minor`.

use std::fmt;

/// The version of the model that this spec defines: 1.0.
pub const MODEL_VERSION: ModelVersion = ModelVersion::new(1, 0);

/// A model version, `major.minor`. A minor version only adds to the model, and a major version
/// may change or remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ModelVersion {
    /// The major version.
    pub major: u16,
    /// The minor version.
    pub minor: u16,
}

impl ModelVersion {
    /// The version `major.minor`.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Whether a core that gives model `self` can run an adapter built for `adapter`: the same
    /// major version, and an adapter minor version that is not later than the core's. An adapter
    /// built for an earlier minor version is accepted, since a minor version only adds; it then
    /// leaves out an item of a kind the later version adds (§8.0 **Coverage report.**).
    #[must_use]
    pub const fn runs(self, adapter: Self) -> bool {
        self.major == adapter.major && adapter.minor <= self.minor
    }
}

impl fmt::Display for ModelVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::{MODEL_VERSION, ModelVersion};

    #[test]
    fn this_spec_defines_model_1_0() {
        assert_eq!(MODEL_VERSION, ModelVersion::new(1, 0));
        assert_eq!(MODEL_VERSION.to_string(), "1.0");
    }

    #[test]
    fn an_adapter_runs_on_the_same_major_and_no_later_minor() {
        let core = ModelVersion::new(1, 2);
        assert!(core.runs(ModelVersion::new(1, 0)));
        assert!(core.runs(ModelVersion::new(1, 2)));
        assert!(!core.runs(ModelVersion::new(1, 3)));
        assert!(!core.runs(ModelVersion::new(2, 0)));
        assert!(!core.runs(ModelVersion::new(0, 9)));
    }
}
