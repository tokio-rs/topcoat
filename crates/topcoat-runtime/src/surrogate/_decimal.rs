use std::cmp::Ordering;

use ref_cast::RefCast;

use crate::{
    BoolSurrogate, impl_surrogate, impl_surrogate_mut, impl_surrogate_ref, serialize_tagged,
};

/// An exact decimal number, backed by a validated numeric string.
///
/// Unlike [`f64`], a `Decimal` never loses precision: it is stored, compared,
/// and displayed as digits, never as a binary float. Use it for money and any
/// other value where a float would be a defect. Arithmetic is deliberately not
/// provided in runtime expressions -- computed values that get stored belong on
/// the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal(String);

impl Decimal {
    /// Creates a decimal from a numeric string (e.g. `"19.99"`, `"-1234.50"`).
    ///
    /// # Panics
    ///
    /// Panics if `value` is not a plain decimal number: an optional leading
    /// `-`, digits, and at most one `.` with digits on both sides.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(is_decimal(&value), "not a decimal number: {value:?}");
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses a numeric string, yielding zero when it is not one. Used by
    /// [`StrSurrogate::to_decimal_or_zero`](crate::StrSurrogate) so a
    /// half-typed input compares as zero instead of panicking mid-keystroke.
    #[must_use]
    pub fn parse_or_zero(value: &str) -> Self {
        if is_decimal(value) {
            Self(value.to_string())
        } else {
            Self("0".to_string())
        }
    }
}

impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The surrogate form of [`Decimal`] used inside runtime expressions.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct DecimalSurrogate(Decimal);

impl DecimalSurrogate {
    #[inline]
    pub(crate) const fn new(v: Decimal) -> Self {
        Self(v)
    }
}

impl_surrogate!(Decimal, DecimalSurrogate);
impl_surrogate_ref!(Decimal, DecimalSurrogate);
impl_surrogate_mut!(Decimal, DecimalSurrogate);

macro_rules! impl_cmp_op {
    ($method:ident, $($ord:ident)|+) => {
        impl DecimalSurrogate {
            /// Compares two decimals by numeric value (so `1.5` equals `1.50`).
            #[inline]
            #[must_use]
            pub fn $method(&self, rhs: &DecimalSurrogate) -> BoolSurrogate {
                BoolSurrogate::new(matches!(cmp(&self.0.0, &rhs.0.0), $(Ordering::$ord)|+))
            }
        }
    };
}

impl_cmp_op!(eq, Equal);
impl_cmp_op!(ne, Less | Greater);
impl_cmp_op!(gt, Greater);
impl_cmp_op!(lt, Less);
impl_cmp_op!(ge, Greater | Equal);
impl_cmp_op!(le, Less | Equal);

impl DecimalSurrogate {
    /// The decimal's exact string form, preserving trailing zeros.
    #[inline]
    #[must_use]
    pub fn to_string(&self) -> crate::StringSurrogate {
        crate::StringSurrogate::new(self.0.0.clone())
    }

    /// Whether the value is exactly zero (any scale: `0`, `0.00`, `-0.0`).
    #[inline]
    #[must_use]
    pub fn is_zero(&self) -> BoolSurrogate {
        BoolSurrogate::new(cmp(&self.0.0, "0") == Ordering::Equal)
    }

    /// Whether the value is strictly less than zero.
    #[inline]
    #[must_use]
    pub fn is_negative(&self) -> BoolSurrogate {
        BoolSurrogate::new(cmp(&self.0.0, "0") == Ordering::Less)
    }
}

impl serde::Serialize for DecimalSurrogate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_tagged(serializer, "Decimal", self.0.as_str())
    }
}

/// Validates the plain-decimal grammar: `-?digits(.digits)?`.
fn is_decimal(s: &str) -> bool {
    let t = s.strip_prefix('-').unwrap_or(s);
    let mut parts = t.split('.');
    let int = parts.next().unwrap_or("");
    let frac = parts.next();
    if parts.next().is_some() {
        return false; // more than one '.'
    }
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    digits(int) && frac.is_none_or(digits)
}

/// Numeric comparison of two validated decimal strings, exact and float-free.
fn cmp(a: &str, b: &str) -> Ordering {
    let (a_neg, a_mag) = split_sign(a);
    let (b_neg, b_mag) = split_sign(b);

    // -0 == 0: a zero magnitude has no sign
    let a_neg = a_neg && !is_zero_mag(a_mag);
    let b_neg = b_neg && !is_zero_mag(b_mag);

    match (a_neg, b_neg) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => cmp_mag(a_mag, b_mag),
        (true, true) => cmp_mag(b_mag, a_mag),
    }
}

fn split_sign(s: &str) -> (bool, &str) {
    match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    }
}

fn is_zero_mag(mag: &str) -> bool {
    mag.bytes().all(|b| b == b'0' || b == b'.')
}

/// Compares two non-negative decimal magnitudes.
fn cmp_mag(a: &str, b: &str) -> Ordering {
    let (a_int, a_frac) = split_point(a);
    let (b_int, b_frac) = split_point(b);

    // integer parts: compare by significant length, then digit-wise
    let a_int = a_int.trim_start_matches('0');
    let b_int = b_int.trim_start_matches('0');
    match a_int.len().cmp(&b_int.len()).then_with(|| a_int.cmp(b_int)) {
        Ordering::Equal => {}
        other => return other,
    }

    // fractional parts: compare digit-wise, the shorter padded with zeros
    let max = a_frac.len().max(b_frac.len());
    let digit = |frac: &str, i: usize| frac.as_bytes().get(i).copied().unwrap_or(b'0');
    for i in 0..max {
        match digit(a_frac, i).cmp(&digit(b_frac, i)) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

fn split_point(s: &str) -> (&str, &str) {
    match s.split_once('.') {
        Some((int, frac)) => (int, frac),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Surrogate;

    fn d(s: &str) -> DecimalSurrogate {
        DecimalSurrogate::new(Decimal::new(s))
    }

    fn t(b: BoolSurrogate) -> bool {
        b.into_real()
    }

    #[test]
    fn scale_insensitive_equality() {
        assert!(t(d("1.5").eq(&d("1.50"))));
        assert!(t(d("1000").eq(&d("1000.00"))));
        assert!(t(d("-0").eq(&d("0"))));
        assert!(t(d("0.0").is_zero()));
    }

    #[test]
    fn ordering_is_numeric_not_lexicographic() {
        // lexicographically "9" > "10", numerically it is not
        assert!(t(d("10").gt(&d("9"))));
        assert!(t(d("1234.50").gt(&d("999.99"))));
        assert!(t(d("0.1").lt(&d("0.11"))));
        assert!(t(d("-5").lt(&d("-4"))));
        assert!(t(d("-1").lt(&d("0.5"))));
        assert!(t(d("-0.01").is_negative()));
    }

    #[test]
    fn display_preserves_trailing_zeros() {
        assert_eq!(d("1234.50").to_string().into_real(), "1234.50");
        assert_eq!(Decimal::new("19.99").to_string(), "19.99");
    }

    #[test]
    #[should_panic(expected = "not a decimal number")]
    fn rejects_non_decimal() {
        let _ = Decimal::new("1.2.3");
    }

    #[test]
    #[should_panic(expected = "not a decimal number")]
    fn rejects_float_notation() {
        let _ = Decimal::new("1e5");
    }
}
