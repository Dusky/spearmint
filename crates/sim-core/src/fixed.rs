//! Q16.16 fixed-point arithmetic.
//!
//! Spec 3.1: integer or fixed-point math only, no floats anywhere in the sim. Floats
//! are not merely discouraged — differences in rounding, FMA contraction, and x87
//! intermediate precision across machines would break byte-identical output, and with
//! it cross-device sync, replays, and anti-cheat.
//!
//! One `Fixed` is an `i32` scaled by 65536, giving a range of about ±32768 with a
//! resolution of about 0.000015. Products and quotients go through `i64` so they
//! cannot overflow mid-computation, and saturate on the way back down.

/// Number of fractional bits.
pub const FRACTIONAL_BITS: u32 = 16;

/// The raw value representing 1.0.
const SCALE: i64 = 1 << FRACTIONAL_BITS;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed(i32);

impl Fixed {
    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(SCALE as i32);
    pub const MIN: Fixed = Fixed(i32::MIN);
    pub const MAX: Fixed = Fixed(i32::MAX);

    /// Wraps a raw scaled value. `Fixed::from_raw(65536)` is 1.0.
    pub const fn from_raw(raw: i32) -> Self {
        Fixed(raw)
    }

    /// The underlying scaled integer.
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Saturates rather than wrapping: a value outside the representable range is a
    /// data error, and clamping it is easier to spot than silent wraparound.
    pub const fn from_int(value: i32) -> Self {
        let scaled = (value as i64) << FRACTIONAL_BITS;
        Fixed(saturate(scaled))
    }

    /// Truncates toward negative infinity.
    pub const fn floor_to_int(self) -> i32 {
        self.0 >> FRACTIONAL_BITS
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub const fn saturating_add(self, other: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(other.0))
    }

    pub const fn saturating_sub(self, other: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(other.0))
    }

    pub const fn saturating_neg(self) -> Fixed {
        Fixed(self.0.saturating_neg())
    }

    /// Multiplication through an `i64` intermediate, truncating toward zero.
    pub const fn saturating_mul(self, other: Fixed) -> Fixed {
        let product = (self.0 as i64) * (other.0 as i64);
        Fixed(saturate(product / SCALE))
    }

    /// Division through an `i64` intermediate. `None` on divide by zero — the caller
    /// decides what a zero denominator means rather than inheriting a panic.
    pub const fn checked_div(self, other: Fixed) -> Option<Fixed> {
        if other.0 == 0 {
            return None;
        }
        let numerator = (self.0 as i64) << FRACTIONAL_BITS;
        Some(Fixed(saturate(numerator / (other.0 as i64))))
    }

    /// Parses a decimal string — `"0.27"`, `"-1.5"`, `"12"` — without ever
    /// constructing a float.
    ///
    /// This is the whole reason the crate carries its own JSON reader. Every JSON
    /// library parses numbers as `f64`, so element data would round-trip through a
    /// float on the way into a sim that is not allowed to contain one.
    pub fn parse(text: &str) -> Result<Fixed, ParseError> {
        let bytes = text.as_bytes();
        let mut index = 0;

        let negative = match bytes.first() {
            Some(b'-') => {
                index += 1;
                true
            }
            Some(b'+') => {
                index += 1;
                false
            }
            _ => false,
        };

        let mut integer_part: i64 = 0;
        let mut saw_digit = false;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            saw_digit = true;
            integer_part = integer_part
                .checked_mul(10)
                .and_then(|value| value.checked_add(i64::from(bytes[index] - b'0')))
                .ok_or(ParseError::OutOfRange)?;
            if integer_part > i64::from(i32::MAX) {
                return Err(ParseError::OutOfRange);
            }
            index += 1;
        }

        // Fractional digits accumulate into a numerator over a power of ten, then get
        // scaled in one integer division. Nine digits is the most an i64 numerator can
        // carry once shifted by 16; more precision than Q16.16 can represent anyway.
        let mut fraction_numerator: i64 = 0;
        let mut fraction_denominator: i64 = 1;
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            let fraction_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                saw_digit = true;
                if fraction_denominator <= 1_000_000_000 {
                    fraction_numerator = fraction_numerator * 10 + i64::from(bytes[index] - b'0');
                    fraction_denominator *= 10;
                }
                index += 1;
            }
            // A decimal point has to be followed by a digit: `1.` is malformed, not one.
            if index == fraction_start {
                return Err(ParseError::Malformed);
            }
        }

        if !saw_digit || index != bytes.len() {
            return Err(ParseError::Malformed);
        }

        let scaled_integer = integer_part
            .checked_shl(FRACTIONAL_BITS)
            .ok_or(ParseError::OutOfRange)?;
        let scaled_fraction = (fraction_numerator << FRACTIONAL_BITS) / fraction_denominator;
        let total = scaled_integer + scaled_fraction;
        let signed = if negative { -total } else { total };

        if signed > i64::from(i32::MAX) || signed < i64::from(i32::MIN) {
            return Err(ParseError::OutOfRange);
        }
        Ok(Fixed(signed as i32))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseError {
    /// Not a decimal number, or has trailing characters. Exponents are not accepted:
    /// element data is hand-written, and `1e3` in a data file is more likely a mistake
    /// than an intent.
    Malformed,
    /// Outside the range Q16.16 can represent.
    OutOfRange,
}

const fn saturate(value: i64) -> i32 {
    if value > i32::MAX as i64 {
        i32::MAX
    } else if value < i32::MIN as i64 {
        i32::MIN
    } else {
        value as i32
    }
}

/// Renders with four decimal places, by integer division — a float would be the
/// obvious way to do this and is exactly what the crate forbids.
impl core::fmt::Display for Fixed {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let negative = self.0 < 0;
        let magnitude = i64::from(self.0).unsigned_abs() as i64;
        let whole = magnitude >> FRACTIONAL_BITS;
        let remainder = magnitude & (SCALE - 1);
        // Four decimal places: scale the remainder by 10^4 before dividing back down.
        let decimals = (remainder * 10_000) / SCALE;
        if negative {
            formatter.write_str("-")?;
        }
        write!(formatter, "{whole}.{decimals:04}")
    }
}

impl core::fmt::Debug for Fixed {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "Fixed({self})")
    }
}
