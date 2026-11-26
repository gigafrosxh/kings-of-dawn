//! Decimal fixed-point numbers.
//!
//! [`Fixed`] stores a signed value scaled by 10^9 in an `i128`. A decimal
//! scale keeps money and percentages exact in base 10, and the 128-bit
//! backing gives roughly ±1.7 × 10^29 integral range, enough for national
//! budgets in any historical currency.
//!
//! Rounding is defined once for the whole simulation: multiplication and
//! division round **half to even** (banker's rounding). All operations are
//! checked; overflow and division by zero are reported, never wrapped.

use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};

/// Number of decimal fraction digits.
pub const FRACTION_DIGITS: u32 = 9;
/// Raw value of `1`.
pub const SCALE: i128 = 1_000_000_000;

/// Arithmetic error of [`Fixed`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedError {
    /// The result does not fit into the representable range.
    Overflow,
    /// Division by zero.
    DivisionByZero,
    /// A decimal string could not be parsed.
    Parse,
}

impl fmt::Display for FixedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Overflow => "fixed-point overflow",
            Self::DivisionByZero => "fixed-point division by zero",
            Self::Parse => "invalid fixed-point literal",
        })
    }
}

impl std::error::Error for FixedError {}

/// Signed decimal fixed-point number with 9 fraction digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed(i128);

impl Fixed {
    /// `0`.
    pub const ZERO: Self = Self(0);
    /// `1`.
    pub const ONE: Self = Self(SCALE);
    /// Largest representable value.
    pub const MAX: Self = Self(i128::MAX);
    /// Smallest representable value.
    pub const MIN: Self = Self(i128::MIN);

    /// Creates a value from its raw scaled representation.
    pub const fn from_raw(raw: i128) -> Self {
        Self(raw)
    }

    /// Raw scaled representation (`value × 10^9`).
    pub const fn raw(self) -> i128 {
        self.0
    }

    /// Converts an integer.
    pub fn from_int(value: i64) -> Self {
        Self(i128::from(value) * SCALE)
    }

    /// `numerator / denominator`, rounded half to even.
    pub fn from_ratio(numerator: i64, denominator: i64) -> Result<Self, FixedError> {
        Self::from_int(numerator).checked_div(Self::from_int(denominator))
    }

    /// Checked addition.
    pub fn checked_add(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(FixedError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(FixedError::Overflow)
    }

    /// Checked negation.
    pub fn checked_neg(self) -> Result<Self, FixedError> {
        self.0.checked_neg().map(Self).ok_or(FixedError::Overflow)
    }

    /// Checked multiplication, rounded half to even.
    pub fn checked_mul(self, rhs: Self) -> Result<Self, FixedError> {
        let product = self.0.checked_mul(rhs.0).ok_or(FixedError::Overflow)?;
        Ok(Self(div_round_half_even(product, SCALE)))
    }

    /// Checked division, rounded half to even.
    pub fn checked_div(self, rhs: Self) -> Result<Self, FixedError> {
        if rhs.0 == 0 {
            return Err(FixedError::DivisionByZero);
        }
        let scaled = self.0.checked_mul(SCALE).ok_or(FixedError::Overflow)?;
        if scaled == i128::MIN && rhs.0 == -1 {
            return Err(FixedError::Overflow);
        }
        Ok(Self(div_round_half_even(scaled, rhs.0)))
    }

    /// Saturating addition (for clamped quantities such as stock levels).
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Absolute value (checked).
    pub fn checked_abs(self) -> Result<Self, FixedError> {
        self.0.checked_abs().map(Self).ok_or(FixedError::Overflow)
    }

    /// Clamps into `[min, max]`.
    pub fn clamp_to(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    /// `true` when below zero.
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Integral part, rounded toward negative infinity.
    pub fn floor_int(self) -> i128 {
        self.0.div_euclid(SCALE)
    }

    /// Rounds to `digits` fraction digits (0..=9), half to even.
    pub fn round_to(self, digits: u32) -> Self {
        let digits = digits.min(FRACTION_DIGITS);
        let step = 10i128.pow(FRACTION_DIGITS - digits);
        Self(div_round_half_even(self.0, step) * step)
    }

    /// Parses a decimal literal such as `-12.5` (at most 9 fraction digits).
    pub fn parse(text: &str) -> Result<Self, FixedError> {
        let (negative, body) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
        if int_part.is_empty()
            || !int_part.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
            || frac_part.len() > FRACTION_DIGITS as usize
            || (body.contains('.') && frac_part.is_empty())
        {
            return Err(FixedError::Parse);
        }
        let int: i128 = int_part.parse().map_err(|_| FixedError::Overflow)?;
        let mut frac: i128 = if frac_part.is_empty() {
            0
        } else {
            frac_part.parse().map_err(|_| FixedError::Parse)?
        };
        frac *= 10i128.pow(FRACTION_DIGITS - frac_part.len() as u32);
        let magnitude = int
            .checked_mul(SCALE)
            .and_then(|v| v.checked_add(frac))
            .ok_or(FixedError::Overflow)?;
        Ok(Self(if negative { -magnitude } else { magnitude }))
    }
}

/// Integer division of `n / d` rounding half to even. `d` must be non-zero.
fn div_round_half_even(n: i128, d: i128) -> i128 {
    let q = n / d; // truncates toward zero
    let r = n % d;
    if r == 0 {
        return q;
    }
    // Compare |2r| with |d| without overflowing: |r| < |d| always holds.
    let r_abs = r.unsigned_abs();
    let d_abs = d.unsigned_abs();
    let twice = r_abs * 2; // r_abs < d_abs <= 2^127, so no u128 overflow
    let away = (n < 0) != (d < 0);
    let step = if away { -1 } else { 1 };
    match twice.cmp(&d_abs) {
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Greater => q + step,
        std::cmp::Ordering::Equal => {
            if q % 2 == 0 {
                q
            } else {
                q + step
            }
        }
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let scale = SCALE as u128;
        let int = abs / scale;
        let frac = abs % scale;
        if frac == 0 {
            write!(f, "{sign}{int}")
        } else {
            let digits = format!("{frac:09}");
            write!(f, "{sign}{int}.{}", digits.trim_end_matches('0'))
        }
    }
}

impl Encode for Fixed {
    fn encode(&self, enc: &mut Encoder) {
        self.0.encode(enc);
    }
}

impl Decode for Fixed {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(i128::decode(dec)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(s: &str) -> Fixed {
        Fixed::parse(s).unwrap()
    }

    #[test]
    fn parse_and_display_roundtrip() {
        for s in [
            "0",
            "1",
            "-1",
            "12.5",
            "-0.000000001",
            "123456789012345678.987654321",
        ] {
            assert_eq!(f(s).to_string(), s);
        }
        for bad in ["", "-", "1.", ".5", "1.0000000001", "1e3", "+1", "1,5"] {
            assert_eq!(Fixed::parse(bad), Err(FixedError::Parse), "{bad}");
        }
    }

    #[test]
    fn multiplication_rounds_half_to_even() {
        // 0.000000001 * 0.5 = 0.0000000005 -> 0 (even)
        assert_eq!(f("0.000000001").checked_mul(f("0.5")).unwrap(), Fixed::ZERO);
        // 0.000000003 * 0.5 = 0.0000000015 -> 0.000000002 (even)
        assert_eq!(
            f("0.000000003").checked_mul(f("0.5")).unwrap(),
            f("0.000000002")
        );
        assert_eq!(
            f("-0.000000003").checked_mul(f("0.5")).unwrap(),
            f("-0.000000002")
        );
        assert_eq!(f("2.5").checked_mul(f("4")).unwrap(), f("10"));
    }

    #[test]
    fn division_is_exact_or_rounded() {
        assert_eq!(Fixed::from_ratio(1, 3).unwrap(), f("0.333333333"));
        assert_eq!(Fixed::from_ratio(2, 3).unwrap(), f("0.666666667"));
        assert_eq!(Fixed::from_ratio(-2, 3).unwrap(), f("-0.666666667"));
        assert_eq!(
            Fixed::ONE.checked_div(Fixed::ZERO),
            Err(FixedError::DivisionByZero)
        );
    }

    #[test]
    fn overflow_is_reported() {
        assert_eq!(
            Fixed::MAX.checked_add(Fixed::ONE),
            Err(FixedError::Overflow)
        );
        assert_eq!(Fixed::MAX.checked_mul(f("2")), Err(FixedError::Overflow));
        assert_eq!(Fixed::MIN.checked_neg(), Err(FixedError::Overflow));
    }

    #[test]
    fn round_to_and_floor() {
        assert_eq!(f("2.345").round_to(2), f("2.34"));
        assert_eq!(f("2.355").round_to(2), f("2.36"));
        assert_eq!(f("-1.5").floor_int(), -2);
        assert_eq!(f("1.5").floor_int(), 1);
    }
}
