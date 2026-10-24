use fixed_decimal::{Decimal, FloatPrecision, SignedRoundingMode, UnsignedRoundingMode};
use icu_decimal::{
    DecimalFormatter, DecimalFormatterPreferences,
    options::{DecimalFormatterOptions, GroupingStrategy},
};
use topcoat_core::context::Cx;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Locale, try_locale};

/// Rounding mode that sends halfway values away from zero, matching
/// `Intl.NumberFormat`.
const ROUNDING: SignedRoundingMode = SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand);

/// Default maximum number of decimal places, matching `Intl.NumberFormat`.
const DEFAULT_MAX_FRACTION_DIGITS: i16 = 3;

/// Maximum decimal places allowed by `Intl.NumberFormat`.
const FRACTION_DIGITS_LIMIT: i16 = 100;

/// Maximum integer or significant digit setting allowed by `Intl.NumberFormat`.
const DIGITS_LIMIT: i16 = 21;

/// A number formatted for a locale, such as `1,234.5` in English or `1.234,5` in German.
///
/// By default, a number shows up to three decimal places and groups its digits. The
/// options follow `Intl.NumberFormat` in JavaScript. In a view, it displays the text for
/// the locale in the current context.
///
/// ```rust
/// use topcoat::l10n::{Number, locale};
///
/// let price = Number::new(1234.5).min_fraction_digits(2);
/// assert_eq!(price.resolve_in(&locale!("en")), "1,234.50");
/// assert_eq!(price.resolve_in(&locale!("de")), "1.234,50");
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Number {
    value: f64,
    // Digit counts are capped at the limits of `Intl.NumberFormat`.
    min_integer_digits: Option<i16>,
    min_fraction_digits: Option<i16>,
    max_fraction_digits: Option<i16>,
    min_significant_digits: Option<i16>,
    max_significant_digits: Option<i16>,
    grouping: bool,
}

impl Number {
    /// Creates a number with the default options.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self {
            value,
            min_integer_digits: None,
            min_fraction_digits: None,
            max_fraction_digits: None,
            min_significant_digits: None,
            max_significant_digits: None,
            grouping: true,
        }
    }

    /// Returns the value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Pads the integer part with leading zeros, as in `007`. Counts above 21 are
    /// treated as 21.
    #[must_use]
    pub const fn min_integer_digits(mut self, count: usize) -> Self {
        self.min_integer_digits = Some(digits(count, DIGITS_LIMIT));
        self
    }

    /// Pads the fraction with trailing zeros, as in `1.50`. Counts above 100 are
    /// treated as 100.
    #[must_use]
    pub const fn min_fraction_digits(mut self, count: usize) -> Self {
        self.min_fraction_digits = Some(digits(count, FRACTION_DIGITS_LIMIT));
        self
    }

    /// Rounds to at most `count` decimal places. The default is three, or the minimum
    /// when that is higher. Counts above 100 are treated as 100.
    #[must_use]
    pub const fn max_fraction_digits(mut self, count: usize) -> Self {
        self.max_fraction_digits = Some(digits(count, FRACTION_DIGITS_LIMIT));
        self
    }

    /// Pads to at least `count` significant digits, as in `5.00` for three. Significant
    /// digit options take precedence over fraction digit options. Counts above 21 are
    /// treated as 21.
    #[must_use]
    pub const fn min_significant_digits(mut self, count: usize) -> Self {
        self.min_significant_digits = Some(digits(count, DIGITS_LIMIT));
        self
    }

    /// Rounds to at most `count` significant digits, as in `1,200` for two. Significant
    /// digit options take precedence over fraction digit options. Counts above 21 are
    /// treated as 21.
    #[must_use]
    pub const fn max_significant_digits(mut self, count: usize) -> Self {
        self.max_significant_digits = Some(digits(count, DIGITS_LIMIT));
        self
    }

    /// Sets whether to separate groups of digits, as in `1,000`. Enabled by default.
    #[must_use]
    pub const fn grouping(mut self, grouping: bool) -> Self {
        self.grouping = grouping;
        self
    }

    /// Returns the text using the [locale](try_locale) stored in `cx`.
    ///
    /// Uses the conventions shared by all languages if the context has no locale.
    #[must_use]
    pub fn resolve(&self, cx: &Cx) -> String {
        self.resolve_in(try_locale(cx).unwrap_or(&Locale::UNKNOWN))
    }

    /// Returns the text for `locale`.
    ///
    /// Infinity and NaN, which have no decimal form, use Rust's formatting.
    #[must_use]
    pub fn resolve_in(&self, locale: &Locale) -> String {
        let locale = locale.clone().into_icu();
        match Self::formatter((&locale).into(), self.grouping) {
            Some(formatter) => self.format_with(&formatter),
            None => self.value.to_string(),
        }
    }

    /// Creates a formatter that separates groups of digits if `grouping` is `true`.
    pub(crate) fn formatter(
        prefs: DecimalFormatterPreferences,
        grouping: bool,
    ) -> Option<DecimalFormatter> {
        let mut options = DecimalFormatterOptions::default();
        if !grouping {
            options.grouping_strategy = Some(GroupingStrategy::Never);
        }
        DecimalFormatter::try_new(prefs, options).ok()
    }

    /// Formats the number with a [`formatter`](Self::formatter) that has the same
    /// grouping setting.
    pub(crate) fn format_with(&self, formatter: &DecimalFormatter) -> String {
        match self.to_decimal() {
            Some(decimal) => formatter.format_to_string(&decimal),
            None => self.value.to_string(),
        }
    }

    /// Converts the value to a decimal with the requested rounding and padding. Returns
    /// `None` for infinity and NaN.
    ///
    /// Significant digits start at the first nonzero digit.
    fn to_decimal(self) -> Option<Decimal> {
        let mut decimal = Decimal::try_from_f64(self.value, FloatPrecision::RoundTrip).ok()?;

        if self.min_significant_digits.is_some() || self.max_significant_digits.is_some() {
            let min = self.min_significant_digits.unwrap_or(1);
            let max = self
                .max_significant_digits
                .unwrap_or(DIGITS_LIMIT)
                .max(min);
            let start = decimal.absolute.nonzero_magnitude_start();
            decimal.round_with_mode(start - max + 1, ROUNDING);
            // Rounding pads zeros up to the rounding position.
            decimal.absolute.trim_end();
            // Rounding can add a digit, as from 999 to 1000.
            let start = decimal.absolute.nonzero_magnitude_start();
            decimal.absolute.pad_end(start - min + 1);
        } else {
            let min = self.min_fraction_digits.unwrap_or(0);
            let max = self
                .max_fraction_digits
                .unwrap_or(DEFAULT_MAX_FRACTION_DIGITS)
                .max(min);
            decimal.round_with_mode(-max, ROUNDING);
            decimal.absolute.trim_end();
            decimal.absolute.pad_end(-min);
        }

        if let Some(min) = self.min_integer_digits {
            decimal.absolute.pad_start(min);
        }
        Some(decimal)
    }
}

/// Converts a digit count to a position offset, without exceeding `limit`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the count is at most `limit`, which is positive"
)]
const fn digits(count: usize, limit: i16) -> i16 {
    if count < limit as usize {
        count as i16
    } else {
        limit
    }
}

macro_rules! from_numbers {
    ($($number:ty)*) => {
        $(
            impl From<$number> for Number {
                #[allow(
                    clippy::cast_lossless,
                    clippy::cast_precision_loss,
                    clippy::unnecessary_cast,
                    reason = "numbers are formatted as `f64`, like in `Intl.NumberFormat`"
                )]
                fn from(number: $number) -> Self {
                    Self::new(number as f64)
                }
            }
        )*
    };
}

from_numbers!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);

/// Displays the number as text in a view.
#[cfg(feature = "view")]
impl NodeViewParts for Number {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}

/// Displays the number in an attribute value, as in `value=(Number::new(1.5))`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for Number {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    fn en(number: Number) -> String {
        number.resolve_in(&locale!("en"))
    }

    #[test]
    fn formats_for_the_locale() {
        let number = Number::new(1234.5);
        assert_eq!(en(number), "1,234.5");
        assert_eq!(number.resolve_in(&locale!("de")), "1.234,5");
    }

    #[test]
    fn formats_for_the_locale_of_the_context() {
        let cx = Cx::default().with(locale!("de"));
        assert_eq!(
            Number::new(1234.5).resolve(&cx),
            Number::new(1234.5).resolve_in(&locale!("de"))
        );
    }

    #[test]
    fn shows_at_most_three_fraction_digits_by_default() {
        assert_eq!(en(Number::new(1.234_56)), "1.235");
    }

    #[test]
    fn rounds_halves_away_from_zero() {
        assert_eq!(en(Number::new(2.5).max_fraction_digits(0)), "3");
        assert_eq!(en(Number::new(-2.5).max_fraction_digits(0)), "-3");
    }

    #[test]
    fn pads_to_the_minimum_digits() {
        assert_eq!(en(Number::new(1.5).min_fraction_digits(2)), "1.50");
        assert_eq!(en(Number::new(7.0).min_integer_digits(3)), "007");
    }

    #[test]
    fn rounds_to_significant_digits() {
        let two = |value| en(Number::new(value).max_significant_digits(2));
        assert_eq!(two(1234.5), "1,200");
        assert_eq!(two(0.000_123), "0.00012");
        assert_eq!(two(999.0), "1,000");
    }

    #[test]
    fn pads_to_significant_digits() {
        assert_eq!(en(Number::new(5.0).min_significant_digits(3)), "5.00");
        assert_eq!(en(Number::new(0.0).min_significant_digits(3)), "0.00");
    }

    #[test]
    fn prefers_significant_digits_over_fraction_digits() {
        let number = Number::new(1234.5)
            .max_significant_digits(2)
            .max_fraction_digits(5);
        assert_eq!(en(number), "1,200");
    }

    #[test]
    fn can_turn_off_grouping() {
        assert_eq!(en(Number::new(1234.5).grouping(false)), "1234.5");
    }

    #[test]
    fn keeps_values_without_a_decimal_form() {
        assert_eq!(en(Number::new(f64::INFINITY)), f64::INFINITY.to_string());
    }

    #[test]
    fn uses_the_numbering_system_of_the_locale() {
        let arabic = Number::new(12.0).resolve_in(&locale!("en-u-nu-arab"));
        assert_ne!(arabic, en(Number::new(12.0)));
    }
}
