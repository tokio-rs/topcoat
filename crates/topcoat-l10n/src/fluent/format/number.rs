use fixed_decimal::{Decimal, FloatPrecision, SignedRoundingMode, UnsignedRoundingMode};
use fluent_bundle::{
    memoizer::MemoizerKind,
    types::{FluentNumber, FluentNumberStyle},
};
use icu_decimal::{
    DecimalFormatter,
    options::{DecimalFormatterOptions, GroupingStrategy},
};
use intl_memoizer::Memoizable;

/// Rounds halves away from zero, as JavaScript's `Intl.NumberFormat` does.
const ROUNDING: SignedRoundingMode = SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand);

/// The number of fraction digits a number shows at most when its options
/// do not say, as in `Intl.NumberFormat`.
const DEFAULT_MAXIMUM_FRACTION_DIGITS: i16 = 3;

/// The most fraction digits `Intl.NumberFormat` accepts.
const FRACTION_DIGITS_LIMIT: i16 = 100;

/// The most integer or significant digits `Intl.NumberFormat` accepts.
const DIGITS_LIMIT: i16 = 21;

/// Formats a number for the locale of `intls`.
///
/// Returns `None` for numbers it does not support yet: currencies,
/// percentages, and numbers without digits such as infinity.
pub(super) fn format_number<M: MemoizerKind>(number: &FluentNumber, intls: &M) -> Option<String> {
    let options = &number.options;
    if options.style != FluentNumberStyle::Decimal {
        return None;
    }
    let decimal = to_decimal(number)?;
    intls
        .with_try_get_threadsafe::<NumberFormatter, _, _>((options.use_grouping,), |formatter| {
            formatter.0.format_to_string(&decimal)
        })
        .ok()
}

/// Converts a Fluent number to a decimal with the digits its options ask for.
///
/// Significant digits count from the first nonzero digit. When they are
/// given, they take precedence over fraction digits.
fn to_decimal(number: &FluentNumber) -> Option<Decimal> {
    let options = &number.options;
    let mut decimal = Decimal::try_from_f64(number.value, FloatPrecision::RoundTrip).ok()?;

    if options.minimum_significant_digits.is_some() || options.maximum_significant_digits.is_some()
    {
        let minimum = options
            .minimum_significant_digits
            .map_or(1, |count| digits(count, DIGITS_LIMIT));
        let maximum = options
            .maximum_significant_digits
            .map_or(DIGITS_LIMIT, |count| digits(count, DIGITS_LIMIT))
            .max(minimum);
        let start = decimal.absolute.nonzero_magnitude_start();
        decimal.round_with_mode(start - maximum + 1, ROUNDING);
        // Rounding can add a digit, as from 999 to 1000.
        let start = decimal.absolute.nonzero_magnitude_start();
        decimal.absolute.pad_end(start - minimum + 1);
    } else {
        let minimum = options
            .minimum_fraction_digits
            .map_or(0, |count| digits(count, FRACTION_DIGITS_LIMIT));
        let maximum = options
            .maximum_fraction_digits
            .map_or(DEFAULT_MAXIMUM_FRACTION_DIGITS, |count| {
                digits(count, FRACTION_DIGITS_LIMIT)
            })
            .max(minimum);
        decimal.round_with_mode(-maximum, ROUNDING);
        decimal.absolute.pad_end(-minimum);
    }

    if let Some(minimum) = options.minimum_integer_digits {
        decimal.absolute.pad_start(digits(minimum, DIGITS_LIMIT));
    }
    Some(decimal)
}

/// Converts a digit count from the options to a position offset, capped at
/// `limit`.
fn digits(count: usize, limit: i16) -> i16 {
    i16::try_from(count).map_or(limit, |count| count.min(limit))
}

/// A number formatter for one locale, kept by a bundle's memoizer.
struct NumberFormatter(DecimalFormatter);

impl Memoizable for NumberFormatter {
    /// Whether to group digits, such as `1,000` instead of `1000`.
    type Args = (bool,);
    type Error = ();

    fn construct(
        lang: unic_langid::LanguageIdentifier,
        (use_grouping,): Self::Args,
    ) -> Result<Self, Self::Error> {
        let locale =
            icu_locale_core::LanguageIdentifier::try_from_str(&lang.to_string()).map_err(|_| ())?;
        let mut options = DecimalFormatterOptions::default();
        if !use_grouping {
            options.grouping_strategy = Some(GroupingStrategy::Never);
        }
        DecimalFormatter::try_new((&locale).into(), options)
            .map(Self)
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};

    use crate::format_value;

    /// Formats the message `n` of `source` in `locale` with `$n` set to `n`.
    fn format(locale: &str, source: &str, n: f64) -> String {
        let resource = FluentResource::try_new(source.to_owned()).unwrap();
        let mut bundle = FluentBundle::new_concurrent(vec![locale.parse().unwrap()]);
        bundle.set_use_isolating(false);
        bundle.set_formatter(Some(format_value));
        bundle.add_builtins().unwrap();
        bundle.add_resource(resource).unwrap();

        let message = bundle.get_message("n").unwrap();
        let mut args = FluentArgs::new();
        args.set("n", n);
        let mut errors = Vec::new();
        let text = bundle.format_pattern(message.value().unwrap(), Some(&args), &mut errors);
        assert!(errors.is_empty(), "{errors:?}");
        text.into_owned()
    }

    #[test]
    fn formats_numbers_for_the_locale() {
        assert_eq!(format("en", "n = { $n }", 1234.5), "1,234.5");
        assert_eq!(format("de", "n = { $n }", 1234.5), "1.234,5");
    }

    #[test]
    fn shows_at_most_three_fraction_digits_by_default() {
        assert_eq!(format("en", "n = { $n }", 1.234_56), "1.235");
    }

    #[test]
    fn rounds_halves_away_from_zero() {
        let source = "n = { NUMBER($n, maximumFractionDigits: 0) }";
        assert_eq!(format("en", source, 2.5), "3");
        assert_eq!(format("en", source, -2.5), "-3");
    }

    #[test]
    fn pads_to_the_minimum_digits() {
        let source = "n = { NUMBER($n, minimumFractionDigits: 2) }";
        assert_eq!(format("de", source, 1.5), "1,50");
        let source = "n = { NUMBER($n, minimumIntegerDigits: 3) }";
        assert_eq!(format("en", source, 7.0), "007");
    }

    #[test]
    fn rounds_to_significant_digits() {
        let source = "n = { NUMBER($n, maximumSignificantDigits: 2) }";
        assert_eq!(format("en", source, 1234.5), "1,200");
        assert_eq!(format("en", source, 0.000_123), "0.00012");
        assert_eq!(format("en", source, 999.0), "1,000");
    }

    #[test]
    fn pads_to_significant_digits() {
        let source = "n = { NUMBER($n, minimumSignificantDigits: 3) }";
        assert_eq!(format("en", source, 5.0), "5.00");
        assert_eq!(format("en", source, 0.0), "0.00");
    }

    #[test]
    fn prefers_significant_digits_over_fraction_digits() {
        let source = "n = { NUMBER($n, maximumSignificantDigits: 2, maximumFractionDigits: 5) }";
        assert_eq!(format("en", source, 1234.5), "1,200");
    }

    #[test]
    fn can_turn_off_grouping() {
        let source = r#"n = { NUMBER($n, useGrouping: "false") }"#;
        assert_eq!(format("de", source, 1234.5), "1234,5");
    }

    #[test]
    fn selects_plural_variants_by_the_number() {
        let source = "n = { $n ->\n    [one] { $n } item\n   *[other] { $n } items\n}\n";
        assert_eq!(format("en", source, 1.0), "1 item");
        assert_eq!(format("en", source, 1000.0), "1,000 items");
    }
}
