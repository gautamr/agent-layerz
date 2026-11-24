//! Numbers with a fraction or an exponent, written as the shortest
//! digits that read back as the same number, in positional form from 1e-6 up to 1e21 and in exponent form outside,
//! `.inf`, `-.inf` and `.nan` for what has no digits, and `-0` for the negative zero.

use std::num::FpCategory;

/// A floating-point number to write. It holds the bits of the number, so two values are equal
/// when they are the same number, a negative zero and a NaN included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmitFloat(u64);

impl EmitFloat {
    /// The value of `number`.
    #[must_use]
    pub const fn new(number: f64) -> Self {
        Self(number.to_bits())
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> f64 {
        f64::from_bits(self.0)
    }

    /// The number as YAML text.
    #[must_use]
    pub fn text(self) -> String {
        let number = self.get();
        match number.classify() {
            FpCategory::Nan => ".nan".to_owned(),
            FpCategory::Infinite if number.is_sign_negative() => "-.inf".to_owned(),
            FpCategory::Infinite => ".inf".to_owned(),
            FpCategory::Zero if number.is_sign_negative() => "-0".to_owned(),
            FpCategory::Zero => "0".to_owned(),
            FpCategory::Normal | FpCategory::Subnormal => {
                let sign = if number.is_sign_negative() { "-" } else { "" };
                format!("{sign}{}", layout(&format!("{:e}", number.abs())))
            }
        }
    }
}

/// The text of a positive number from its shortest scientific form (`1.2345e2`, `5e-1`), laid out
/// as ECMAScript's `Number::toString` lays out `k` digits with the decimal point after `n` of
/// them: whole numbers up to 21 digits and fractions down to 6 zeros in plain notation, the rest
/// as `d.ddde+x`.
fn layout(scientific: &str) -> String {
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let point = exponent.saturating_add(1);
    let zeros = |how_many: i32| "0".repeat(usize::try_from(how_many).unwrap_or(0));
    let before = |length: i32| -> String {
        digits
            .chars()
            .take(usize::try_from(length).unwrap_or(0))
            .collect()
    };
    let after = |length: i32| -> String {
        digits
            .chars()
            .skip(usize::try_from(length).unwrap_or(0))
            .collect()
    };
    if count <= point && point <= 21 {
        format!("{digits}{}", zeros(point - count))
    } else if 0 < point && point <= 21 {
        format!("{}.{}", before(point), after(point))
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", zeros(-point))
    } else {
        let power = point - 1;
        let sign = if power < 0 { '-' } else { '+' };
        let head = before(1);
        let tail = after(1);
        let fraction = if tail.is_empty() {
            String::new()
        } else {
            format!(".{tail}")
        };
        format!("{head}{fraction}e{sign}{}", power.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::EmitFloat;

    fn written(number: f64) -> String {
        EmitFloat::new(number).text()
    }

    /// Each pair is a number and the text spec §6 gives for it.
    #[test]
    fn numbers_are_written_as_the_shortest_decimal_that_reads_back() {
        for (number, text) in [
            (0.5, "0.5"),
            (-0.5, "-0.5"),
            (1.0, "1"),
            (100.25, "100.25"),
            (1e20, "100000000000000000000"),
            (1.2345e20, "123450000000000000000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (123_456_789_012_345_680_000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1.234e-6, "0.000001234"),
            (1e-7, "1e-7"),
            (-1.5e-10, "-1.5e-10"),
            (1.5e300, "1.5e+300"),
            (5e-324, "5e-324"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::MAX, "1.7976931348623157e+308"),
            (0.0, "0"),
            (-0.0, "-0"),
            (f64::INFINITY, ".inf"),
            (f64::NEG_INFINITY, "-.inf"),
            (f64::NAN, ".nan"),
        ] {
            assert_eq!(written(number), text, "{number:?}");
        }
    }

    #[test]
    fn two_floats_are_equal_when_they_are_the_same_number() {
        assert_eq!(EmitFloat::new(0.5), EmitFloat::new(0.5));
        assert_ne!(EmitFloat::new(0.0), EmitFloat::new(-0.0));
        assert_eq!(EmitFloat::new(f64::NAN), EmitFloat::new(f64::NAN));
    }
}
