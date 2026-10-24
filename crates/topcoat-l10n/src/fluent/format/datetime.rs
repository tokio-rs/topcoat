use std::borrow::Cow;

use fluent_bundle::{
    FluentArgs, FluentValue,
    types::{AnyEq, FluentType},
};
use icu_datetime::{DateTimeFormatter, fieldsets::enums::CompositeDateTimeFieldSet};
use intl_memoizer::{IntlLangMemoizer, Memoizable, concurrent};

use crate::{DateTime, DateTimeStyle, datetime::Styles};

/// A [`DateTime`] passed to Fluent, which formats it with the bundle's cached formatter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FluentDateTime(pub(crate) DateTime);

impl FluentType for FluentDateTime {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(*self)
    }

    fn as_string(&self, intls: &IntlLangMemoizer) -> Cow<'static, str> {
        let text = intls
            .with_try_get::<DateTimeFormatterMemo, _, _>((self.0.styles(),), |formatter| {
                self.0.format_with(&formatter.0)
            });
        Cow::Owned(text.unwrap_or_else(|()| self.0.fallback()))
    }

    fn as_string_threadsafe(&self, intls: &concurrent::IntlLangMemoizer) -> Cow<'static, str> {
        let text = intls
            .with_try_get::<DateTimeFormatterMemo, _, _>((self.0.styles(),), |formatter| {
                self.0.format_with(&formatter.0)
            });
        Cow::Owned(text.unwrap_or_else(|()| self.0.fallback()))
    }
}

/// Implements the Fluent function `DATETIME`, which sets the `dateStyle` and
/// `timeStyle` options of `Intl.DateTimeFormat` on a date or time.
///
/// Returns an error value if the first argument is not a date or time. Ignores other
/// options and unknown styles.
#[allow(non_snake_case, reason = "Fluent functions are uppercase")]
pub(crate) fn DATETIME<'a>(positional: &[FluentValue<'a>], named: &FluentArgs) -> FluentValue<'a> {
    let Some(FluentValue::Custom(value)) = positional.first() else {
        return FluentValue::Error;
    };
    // `as_any` must be called on the boxed value, not on the box.
    let Some(&FluentDateTime(mut value)) = AnyEq::as_any(&**value).downcast_ref() else {
        return FluentValue::Error;
    };
    for (name, option) in named.iter() {
        let FluentValue::String(style) = option else {
            continue;
        };
        let Some(style) = DateTimeStyle::from_intl(style) else {
            continue;
        };
        value = match name {
            "dateStyle" => value.date_style(style),
            "timeStyle" => value.time_style(style),
            _ => value,
        };
    }
    FluentValue::Custom(Box::new(FluentDateTime(value)))
}

impl DateTimeStyle {
    /// Parses a style name of `Intl.DateTimeFormat`, such as `"long"`.
    fn from_intl(name: &str) -> Option<Self> {
        match name {
            "full" => Some(Self::Full),
            "long" => Some(Self::Long),
            "medium" => Some(Self::Medium),
            "short" => Some(Self::Short),
            _ => None,
        }
    }
}

/// A cached date and time formatter for one bundle locale.
struct DateTimeFormatterMemo(DateTimeFormatter<CompositeDateTimeFieldSet>);

impl Memoizable for DateTimeFormatterMemo {
    /// The date and time styles to show.
    type Args = (Styles,);
    type Error = ();

    fn construct(
        lang: unic_langid::LanguageIdentifier,
        (styles,): Self::Args,
    ) -> Result<Self, Self::Error> {
        let locale =
            icu_locale_core::LanguageIdentifier::try_from_str(&lang.to_string()).map_err(|_| ())?;
        DateTime::formatter((&locale).into(), styles)
            .map(Self)
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Argument, Catalog, DateTime,
        DateTimeStyle::{Long, Short},
        locale,
    };

    const EN: &str = "\
plain = { $date }
long = { DATETIME($date, dateStyle: \"long\") }
both = { DATETIME($date, dateStyle: \"short\", timeStyle: \"short\") }
not-a-date = { DATETIME($count) }
";

    fn format(id: &str, date: DateTime) -> String {
        let catalog = Catalog::new([(locale!("en"), &[EN][..])]);
        let args = [("date", Argument::from(date)), ("count", Argument::from(1))];
        catalog
            .format(&locale!("en"), id, None, &args)
            .expect("the message exists")
            .into_owned()
    }

    fn date() -> DateTime {
        DateTime::date(2026, 10, 12).unwrap().at(14, 5, 0).unwrap()
    }

    /// Strips the isolation marks Fluent puts around arguments.
    fn text(id: &str, date: DateTime) -> String {
        format(id, date).replace(['\u{2068}', '\u{2069}'], "")
    }

    #[test]
    fn formats_like_a_date_time_with_the_same_styles() {
        let en = locale!("en");
        assert_eq!(text("plain", date()), date().resolve_in(&en));
        assert_eq!(text("long", date()), date().date_style(Long).resolve_in(&en));
        assert_eq!(
            text("both", date()),
            date().date_style(Short).time_style(Short).resolve_in(&en)
        );
    }

    #[test]
    fn rejects_values_that_are_not_dates() {
        assert_ne!(text("not-a-date", date()), "1");
    }
}
