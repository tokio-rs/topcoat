use fluent_bundle::{
    memoizer::MemoizerKind,
    types::{FluentNumber, FluentNumberStyle},
};
use icu_decimal::DecimalFormatter;
use intl_memoizer::Memoizable;

use crate::Number;

/// Formats a number using the locale stored in `intls`.
///
/// Returns `None` for unsupported values, including currency and percentages.
pub(super) fn format_number<M: MemoizerKind>(number: &FluentNumber, intls: &M) -> Option<String> {
    let grouping = number.options.use_grouping;
    let number = Number::from_fluent(number)?;
    intls
        .with_try_get_threadsafe::<NumberFormatter, _, _>((grouping,), |formatter| {
            number.format_with(&formatter.0)
        })
        .ok()
}

impl Number {
    /// Converts a Fluent number with `NUMBER` options, or returns `None` for styles
    /// other than decimal.
    fn from_fluent(number: &FluentNumber) -> Option<Self> {
        let options = &number.options;
        if options.style != FluentNumberStyle::Decimal {
            return None;
        }
        let mut result = Self::new(number.value).grouping(options.use_grouping);
        if let Some(count) = options.minimum_integer_digits {
            result = result.min_integer_digits(count);
        }
        if let Some(count) = options.minimum_fraction_digits {
            result = result.min_fraction_digits(count);
        }
        if let Some(count) = options.maximum_fraction_digits {
            result = result.max_fraction_digits(count);
        }
        if let Some(count) = options.minimum_significant_digits {
            result = result.min_significant_digits(count);
        }
        if let Some(count) = options.maximum_significant_digits {
            result = result.max_significant_digits(count);
        }
        Some(result)
    }
}

/// A cached number formatter for one bundle locale.
struct NumberFormatter(DecimalFormatter);

impl Memoizable for NumberFormatter {
    /// Whether to insert digit separators, as in `1,000`.
    type Args = (bool,);
    type Error = ();

    fn construct(
        lang: unic_langid::LanguageIdentifier,
        (grouping,): Self::Args,
    ) -> Result<Self, Self::Error> {
        let locale =
            icu_locale_core::LanguageIdentifier::try_from_str(&lang.to_string()).map_err(|_| ())?;
        Number::formatter((&locale).into(), grouping)
            .map(Self)
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};

    use crate::format_value;

    /// Formats the test message `n` in the given locale with `$n` set to the supplied
    /// number.
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
    fn applies_the_options_of_number() {
        let source = "n = { NUMBER($n, minimumIntegerDigits: 2, minimumFractionDigits: 2) }";
        assert_eq!(format("en", source, 7.0), "07.00");
        let source = "n = { NUMBER($n, maximumFractionDigits: 0) }";
        assert_eq!(format("en", source, 2.5), "3");
        let source = "n = { NUMBER($n, minimumSignificantDigits: 3, maximumSignificantDigits: 3) }";
        assert_eq!(format("en", source, 1234.5), "1,230");
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
