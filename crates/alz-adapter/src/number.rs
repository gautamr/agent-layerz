//! A float that JSON and TOML can write: finite.

use thiserror::Error;

/// A finite floating point number: neither infinite nor NaN. Neither JSON nor TOML can write the
/// others, and a finite value is equal to itself, which an `f64` is not always.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finite(f64);

/// A float that is infinite or not a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a finite number")]
pub struct NotFinite;

impl Finite {
    /// The number as an `f64`.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Finite {
    type Error = NotFinite;

    fn try_from(number: f64) -> Result<Self, Self::Error> {
        if number.is_finite() {
            Ok(Self(number))
        } else {
            Err(NotFinite)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Finite, NotFinite};

    #[test]
    fn only_a_finite_float_is_a_finite_number() {
        assert_eq!(Finite::try_from(1.5).map(Finite::get), Ok(1.5));
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Finite::try_from(number), Err(NotFinite));
        }
    }
}
