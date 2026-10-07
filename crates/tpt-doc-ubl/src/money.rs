//! Exact decimal types for invoice arithmetic.
//!
//! Binary floating point cannot represent `0.1`, so `3 × 0.10` is not `0.30`
//! and a total can differ from the sum of its parts by a cent. EN 16931
//! arithmetic rules compare amounts for *equality*, so the model uses exact
//! types instead:
//!
//! - [`Money`] — a signed amount in **minor units with two decimals** (cents).
//!   This matches the two-decimal amounts of PEPPOL BIS Billing 3.0; currencies
//!   with a different minor unit (JPY, KWD) are not modelled yet.
//! - [`Decimal`] — an exact base-10 number for quantities, unit prices and tax
//!   rates, which may carry more than two decimals.
//!
//! Rounding is half away from zero, the convention EN 16931 uses.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tpt_doc_core::DocError;

/// Most fractional digits a [`Decimal`] may carry.
const MAX_SCALE: u8 = 12;
/// Most significant digits accepted when parsing.
const MAX_DIGITS: usize = 30;

/// A signed monetary amount with exactly two decimal places.
///
/// Addition saturates instead of wrapping: a saturated total can never equal
/// the sum a real document declares, so validation reports it rather than
/// accepting a silently wrapped value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(i64);

impl Money {
    /// Zero.
    pub const ZERO: Self = Self(0);

    /// An amount from a count of cents (`12345` is `123.45`).
    #[must_use]
    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    /// The amount as a count of cents.
    #[must_use]
    pub const fn cents(self) -> i64 {
        self.0
    }

    /// Saturating addition.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Parse a decimal string such as `-12.30`.
    ///
    /// More than two fractional digits are accepted only when the extra digits
    /// are zero (`12.300`), because anything else cannot be represented.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] for malformed text, a value that
    /// does not fit, or a non-zero digit beyond the second decimal.
    pub fn parse(text: &str) -> Result<Self, DocError> {
        let decimal = Decimal::parse(text)?;
        match decimal.to_money_exact() {
            Some(money) => Ok(money),
            None => Err(DocError::invalid_format(format!(
                "`{}` is not an amount with at most two decimals",
                text.trim()
            ))),
        }
    }
}

impl std::ops::Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl std::ops::Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, std::ops::Add::add)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

impl FromStr for Money {
    type Err = DocError;
    fn from_str(s: &str) -> Result<Self, DocError> {
        Self::parse(s)
    }
}

impl Serialize for Money {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// An exact base-10 number: `mantissa / 10^scale`.
///
/// Equality and ordering compare *values*, so `15` equals `15.00`.
#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    mantissa: i128,
    scale: u8,
}

impl Decimal {
    /// Zero.
    pub const ZERO: Self = Self {
        mantissa: 0,
        scale: 0,
    };

    /// `mantissa × 10^-scale`, e.g. `Decimal::new(999, 2)` is `9.99`.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if `scale` exceeds 12.
    pub fn new(mantissa: i64, scale: u8) -> Result<Self, DocError> {
        if scale > MAX_SCALE {
            return Err(DocError::invalid_format(format!(
                "decimal scale {scale} exceeds {MAX_SCALE}"
            )));
        }
        Ok(Self {
            mantissa: i128::from(mantissa),
            scale,
        })
    }

    /// A whole number.
    #[must_use]
    pub fn from_int(value: i64) -> Self {
        Self {
            mantissa: i128::from(value),
            scale: 0,
        }
    }

    /// Parse plain decimal notation (`-12`, `0.125`). Exponents, signs other
    /// than a leading `+`/`-`, and more than 12 fractional digits are rejected.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] describing what was wrong.
    pub fn parse(text: &str) -> Result<Self, DocError> {
        let trimmed = text.trim();
        let bad =
            |why: &str| DocError::invalid_format(format!("`{trimmed}` is not a number: {why}"));
        let (negative, body) = match trimmed.as_bytes().first() {
            Some(b'-') => (true, &trimmed[1..]),
            Some(b'+') => (false, &trimmed[1..]),
            _ => (false, trimmed),
        };
        let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(bad("no digits"));
        }
        if !int_part.bytes().all(|b| b.is_ascii_digit())
            || !frac_part.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(bad("only digits and one decimal point are allowed"));
        }
        let frac_part = frac_part.trim_end_matches('0');
        let scale = u8::try_from(frac_part.len())
            .ok()
            .filter(|s| *s <= MAX_SCALE)
            .ok_or_else(|| bad("too many decimal places"))?;
        if int_part.len() + frac_part.len() > MAX_DIGITS {
            return Err(bad("too many digits"));
        }
        let digits = format!("{int_part}{frac_part}");
        let magnitude: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse().map_err(|_| bad("out of range"))?
        };
        Ok(Self {
            mantissa: if negative { -magnitude } else { magnitude },
            scale,
        })
    }

    /// Exact product, or `None` on overflow.
    #[must_use]
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let scale = self.scale.checked_add(other.scale)?;
        let mantissa = self.mantissa.checked_mul(other.mantissa)?;
        Some(Self { mantissa, scale }.reduced())
    }

    /// Strip trailing fractional zeros so equal values share a representation.
    fn reduced(mut self) -> Self {
        while self.scale > 0 && self.mantissa % 10 == 0 {
            self.mantissa /= 10;
            self.scale -= 1;
        }
        self
    }

    /// Round half away from zero to cents. `None` if the result overflows.
    #[must_use]
    pub fn round_to_money(self) -> Option<Money> {
        self.round_cents_divided_by(1)
    }

    /// `self / divisor` rounded half away from zero to cents (`divisor > 0`).
    fn round_cents_divided_by(self, divisor: u32) -> Option<Money> {
        // value * 100 / divisor, as an exact integer ratio num / den.
        let mut num = self.mantissa.checked_mul(100)?;
        let mut den: i128 = 10_i128.checked_pow(u32::from(self.scale))?;
        den = den.checked_mul(i128::from(divisor))?;
        // Round half away from zero.
        let negative = num < 0;
        if negative {
            num = -num;
        }
        let quotient = num / den;
        let remainder = num % den;
        let rounded = if remainder * 2 >= den {
            quotient + 1
        } else {
            quotient
        };
        let rounded = if negative { -rounded } else { rounded };
        i64::try_from(rounded).ok().map(Money)
    }

    /// `amount × self / 100` rounded to cents — a percentage `self` of `amount`.
    #[must_use]
    pub fn percent_of(self, amount: Money) -> Option<Money> {
        let product = self.checked_mul(Decimal {
            mantissa: i128::from(amount.0),
            scale: 2,
        })?;
        product.round_cents_divided_by(100)
    }

    /// `Some(money)` when the value is exactly representable in cents.
    fn to_money_exact(self) -> Option<Money> {
        let reduced = self.reduced();
        if reduced.scale > 2 {
            return None;
        }
        let cents = reduced
            .mantissa
            .checked_mul(10_i128.pow(u32::from(2 - reduced.scale)))?;
        i64::try_from(cents).ok().map(Money)
    }

    /// Render with at least `min_scale` fractional digits (`100` -> `100.00`).
    #[must_use]
    pub fn to_string_min_scale(self, min_scale: u8) -> String {
        let d = self.reduced();
        let mut text = d.to_string();
        let have = if d.scale == 0 { 0 } else { d.scale };
        if have < min_scale {
            if have == 0 {
                text.push('.');
            }
            text.push_str(&"0".repeat(usize::from(min_scale - have)));
        }
        text
    }

    fn cmp_value(&self, other: &Self) -> Ordering {
        let (a, b) = (self.reduced(), other.reduced());
        let target = a.scale.max(b.scale);
        let lift = |d: Self| {
            d.mantissa
                .checked_mul(10_i128.pow(u32::from(target - d.scale)))
        };
        match (lift(a), lift(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            // A lift only overflows for magnitudes beyond any real quantity;
            // fall back to the sign so the ordering stays total.
            _ => a.mantissa.signum().cmp(&b.mantissa.signum()),
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_value(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_value(other)
    }
}

impl Default for Decimal {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<i32> for Decimal {
    fn from(value: i32) -> Self {
        Self::from_int(i64::from(value))
    }
}

impl From<u32> for Decimal {
    fn from(value: u32) -> Self {
        Self::from_int(i64::from(value))
    }
}

impl From<Money> for Decimal {
    fn from(money: Money) -> Self {
        Self {
            mantissa: i128::from(money.0),
            scale: 2,
        }
        .reduced()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = self.reduced();
        let sign = if d.mantissa < 0 { "-" } else { "" };
        let abs = d.mantissa.unsigned_abs();
        if d.scale == 0 {
            return write!(f, "{sign}{abs}");
        }
        let pow = 10_u128.pow(u32::from(d.scale));
        write!(
            f,
            "{sign}{}.{:0width$}",
            abs / pow,
            abs % pow,
            width = usize::from(d.scale)
        )
    }
}

impl FromStr for Decimal {
    type Err = DocError;
    fn from_str(s: &str) -> Result<Self, DocError> {
        Self::parse(s)
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }
    fn m(s: &str) -> Money {
        s.parse().expect("money")
    }

    #[test]
    fn three_times_ten_cents_is_exactly_thirty_cents() {
        // In f64, 3.0 * 0.1 == 0.30000000000000004.
        let line = d("3")
            .checked_mul(d("0.10"))
            .unwrap()
            .round_to_money()
            .unwrap();
        assert_eq!(line, m("0.30"));
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(d("0.335").round_to_money().unwrap(), m("0.34"));
        assert_eq!(d("-0.335").round_to_money().unwrap(), m("-0.34"));
        assert_eq!(d("10.005").round_to_money().unwrap(), m("10.01"));
        assert_eq!(d("0.004").round_to_money().unwrap(), m("0.00"));
        // 3 x 0.335 = 1.005 -> 1.01 (f64 gives 1.00 on some inputs)
        let p = d("3").checked_mul(d("0.335")).unwrap();
        assert_eq!(p.round_to_money().unwrap(), m("1.01"));
    }

    #[test]
    fn percentage_of_an_amount() {
        assert_eq!(d("15").percent_of(m("420.00")).unwrap(), m("63.00"));
        assert_eq!(d("15").percent_of(m("0.30")).unwrap(), m("0.05")); // 0.045 -> 0.05
        assert_eq!(d("12.5").percent_of(m("-8.00")).unwrap(), m("-1.00"));
        assert_eq!(d("0").percent_of(m("99.99")).unwrap(), Money::ZERO);
    }

    #[test]
    fn money_parse_and_display() {
        assert_eq!(m("12").to_string(), "12.00");
        assert_eq!(m("-0.5").to_string(), "-0.50");
        assert_eq!(m("12.300").cents(), 1230);
        assert_eq!(m(" +7.05 ").cents(), 705);
        for bad in [
            "", ".", "1.234", "1e3", "abc", "1,5", "--1", "NaN", "inf", "1.2.3",
        ] {
            assert!(Money::parse(bad).is_err(), "{bad:?}");
        }
        assert!(Money::parse("99999999999999999999").is_err());
        assert_eq!(
            Money::from_cents(i64::MIN).to_string(),
            "-92233720368547758.08"
        );
    }

    #[test]
    fn decimal_equality_is_by_value() {
        assert_eq!(d("15"), d("15.0"));
        assert_eq!(d("15.00").to_string(), "15");
        assert_eq!(d("0.50").to_string(), "0.5");
        assert!(d("2") < d("10"));
        assert!(d("-1") < d("0.001"));
        assert_eq!(d("-0"), d("0"));
        assert!(Decimal::parse("1.0000000000001").is_err());
        assert!(Decimal::parse(&"9".repeat(40)).is_err());
    }

    #[test]
    fn arithmetic_saturates_instead_of_wrapping() {
        let max = Money::from_cents(i64::MAX);
        assert_eq!(max + Money::from_cents(1), max);
        assert_eq!([max, max].into_iter().sum::<Money>(), max);
        assert!(d("9999999999").checked_mul(d("9999999999")).is_some());
        assert!(
            d("9999999999")
                .checked_mul(d("9999999999"))
                .unwrap()
                .round_to_money()
                .is_none()
        );
    }

    #[test]
    fn serde_round_trips_as_strings() {
        let json = serde_json::to_string(&(m("1.50"), d("2.250"))).unwrap();
        assert_eq!(json, r#"["1.50","2.25"]"#);
        let back: (Money, Decimal) = serde_json::from_str(&json).unwrap();
        assert_eq!(back, (m("1.50"), d("2.25")));
    }
}
