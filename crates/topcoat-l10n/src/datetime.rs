use icu_calendar::Iso;
use icu_datetime::{
    DateTimeFormatter, DateTimeFormatterPreferences,
    fieldsets::{
        builder::{DateFields, FieldSetBuilder},
        enums::CompositeDateTimeFieldSet,
    },
    input::{self, Date, Time},
    options::{Length, TimePrecision},
};
use topcoat_core::context::Cx;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Locale, try_locale};

/// A date, a time of day or both, formatted for a locale, such as `Oct 12, 2026, 2:05 PM`
/// in English or `12.10.2026, 14:05` in German.
///
/// Without styles, a date uses [`DateTimeStyle::Medium`] and a time uses
/// [`DateTimeStyle::Short`]. Dates use the locale's calendar. In a view, it displays the
/// text for the locale in the current context.
///
/// ```rust
/// use topcoat::l10n::{DateTime, DateTimeStyle, locale};
///
/// let date = DateTime::date(2026, 10, 12).unwrap();
/// assert_eq!(date.resolve_in(&locale!("en")), "Oct 12, 2026");
///
/// let long = date.date_style(DateTimeStyle::Long);
/// assert_eq!(long.resolve_in(&locale!("de")), "12. Oktober 2026");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    value: Value,
    date_style: Option<DateTimeStyle>,
    time_style: Option<DateTimeStyle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    Date(Date<Iso>),
    Time(Time),
    DateTime(Date<Iso>, Time),
}

impl DateTime {
    /// Creates a date in the ISO calendar, or returns `None` if the month or day is out
    /// of range.
    #[must_use]
    pub fn date(year: i32, month: u8, day: u8) -> Option<Self> {
        Date::try_new_iso(year, month, day).ok().map(Self::from)
    }

    /// Creates a time of day, or returns `None` if a field is out of range.
    #[must_use]
    pub fn time(hour: u8, minute: u8, second: u8) -> Option<Self> {
        Time::try_new(hour, minute, second, 0).ok().map(Self::from)
    }

    /// Sets the time of day, as in `DateTime::date(2026, 10, 12)?.at(14, 5, 0)`. Returns
    /// `None` if a field is out of range.
    #[must_use]
    pub fn at(self, hour: u8, minute: u8, second: u8) -> Option<Self> {
        let time = Time::try_new(hour, minute, second, 0).ok()?;
        let value = match self.value {
            Value::Date(date) | Value::DateTime(date, _) => Value::DateTime(date, time),
            Value::Time(_) => Value::Time(time),
        };
        Some(Self { value, ..self })
    }

    const fn new(value: Value) -> Self {
        Self {
            value,
            date_style: None,
            time_style: None,
        }
    }

    /// Sets how to show the date. Has no effect on a time of day.
    #[must_use]
    pub const fn date_style(mut self, style: DateTimeStyle) -> Self {
        self.date_style = Some(style);
        self
    }

    /// Sets how to show the time of day. Has no effect on a date.
    #[must_use]
    pub const fn time_style(mut self, style: DateTimeStyle) -> Self {
        self.time_style = Some(style);
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
    #[must_use]
    pub fn resolve_in(&self, locale: &Locale) -> String {
        let locale = locale.clone().into_icu();
        match Self::formatter((&locale).into(), self.styles()) {
            Some(formatter) => self.format_with(&formatter),
            None => self.fallback(),
        }
    }

    /// Returns the date and time styles to show, filling in the defaults.
    ///
    /// Styles for fields the value lacks are dropped. If none of the requested fields
    /// exist, the defaults apply.
    pub(crate) fn styles(&self) -> Styles {
        let (has_date, has_time) = match self.value {
            Value::Date(_) => (true, false),
            Value::Time(_) => (false, true),
            Value::DateTime(..) => (true, true),
        };
        let requested = Styles {
            date: self.date_style.filter(|_| has_date),
            time: self.time_style.filter(|_| has_time),
        };
        if requested.date.is_some() || requested.time.is_some() {
            requested
        } else {
            Styles {
                date: has_date.then_some(DateTimeStyle::Medium),
                time: has_time.then_some(DateTimeStyle::Short),
            }
        }
    }

    /// Creates a formatter that shows the fields in `styles`.
    pub(crate) fn formatter(
        prefs: DateTimeFormatterPreferences,
        styles: Styles,
    ) -> Option<DateTimeFormatter<CompositeDateTimeFieldSet>> {
        let mut builder = FieldSetBuilder::new();
        builder.length = Some(match styles.date {
            Some(DateTimeStyle::Full | DateTimeStyle::Long) => Length::Long,
            Some(DateTimeStyle::Short) => Length::Short,
            Some(DateTimeStyle::Medium) | None => Length::Medium,
        });
        builder.date_fields = styles.date.map(|style| match style {
            DateTimeStyle::Full => DateFields::YMDE,
            _ => DateFields::YMD,
        });
        builder.time_precision = styles.time.map(|style| match style {
            DateTimeStyle::Short => TimePrecision::Minute,
            _ => TimePrecision::Second,
        });
        let field_set = builder.build_composite_datetime().ok()?;
        DateTimeFormatter::try_new(prefs, field_set).ok()
    }

    /// Formats the value with a [`formatter`](Self::formatter) for its
    /// [styles](Self::styles).
    pub(crate) fn format_with(&self, formatter: &DateTimeFormatter<CompositeDateTimeFieldSet>) -> String {
        // The formatter only shows the fields of the styles, so the missing half can be
        // any valid value.
        let (date, time) = match self.value {
            Value::Date(date) => (date, Time::start_of_day()),
            Value::Time(time) => (
                Date::try_new_iso(2000, 1, 1).expect("2000-01-01 is a valid date"),
                time,
            ),
            Value::DateTime(date, time) => (date, time),
        };
        formatter
            .format(&input::DateTime { date, time })
            .to_string()
    }

    /// Returns ISO 8601 text for when no formatter is available.
    pub(crate) fn fallback(&self) -> String {
        let date = |date: &Date<Iso>| {
            format!(
                "{:04}-{:02}-{:02}",
                date.year().extended_year(),
                date.month().ordinal,
                date.day_of_month().0
            )
        };
        let time = |time: &Time| {
            format!(
                "{:02}:{:02}:{:02}",
                time.hour.number(),
                time.minute.number(),
                time.second.number()
            )
        };
        match &self.value {
            Value::Date(value) => date(value),
            Value::Time(value) => time(value),
            Value::DateTime(day, clock) => format!("{}T{}", date(day), time(clock)),
        }
    }
}

impl From<Date<Iso>> for DateTime {
    fn from(date: Date<Iso>) -> Self {
        Self::new(Value::Date(date))
    }
}

impl From<Time> for DateTime {
    fn from(time: Time) -> Self {
        Self::new(Value::Time(time))
    }
}

impl From<input::DateTime<Iso>> for DateTime {
    fn from(value: input::DateTime<Iso>) -> Self {
        Self::new(Value::DateTime(value.date, value.time))
    }
}

/// How much detail to show for a date or a time of day, following `dateStyle` and
/// `timeStyle` of `Intl.DateTimeFormat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateTimeStyle {
    /// Spells out the date with the weekday, as in `Monday, October 12, 2026`. Shows
    /// times like [`Long`](Self::Long).
    Full,
    /// Spells out the date, as in `October 12, 2026`, and shows a time with seconds.
    Long,
    /// Abbreviates the date, as in `Oct 12, 2026`, and shows a time with seconds.
    Medium,
    /// Shows a numeric date, as in `10/12/26`, and a time without seconds, as in
    /// `2:05 PM`.
    Short,
}

/// The resolved styles of a [`DateTime`]. `None` hides that part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Styles {
    pub(crate) date: Option<DateTimeStyle>,
    pub(crate) time: Option<DateTimeStyle>,
}

/// Displays the date and time as text in a view.
#[cfg(feature = "view")]
impl NodeViewParts for DateTime {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}

/// Displays the date and time in an attribute value, as in `title=(date)`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for DateTime {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string(self.resolve(cx));
    }
}

#[cfg(feature = "jiff")]
mod jiff_impls {
    use jiff::{Timestamp, Zoned, civil, tz::TimeZone};

    use super::{Date, DateTime, Time, Value};

    fn date(date: civil::Date) -> Date<icu_calendar::Iso> {
        Date::try_new_iso(
            i32::from(date.year()),
            date.month().unsigned_abs(),
            date.day().unsigned_abs(),
        )
        .expect("a `jiff` date is a valid ISO date")
    }

    fn time(time: civil::Time) -> Time {
        Time::try_new(
            time.hour().unsigned_abs(),
            time.minute().unsigned_abs(),
            time.second().unsigned_abs(),
            time.subsec_nanosecond().unsigned_abs(),
        )
        .expect("a `jiff` time is a valid time of day")
    }

    impl From<civil::Date> for DateTime {
        fn from(value: civil::Date) -> Self {
            Self::new(Value::Date(date(value)))
        }
    }

    impl From<civil::Time> for DateTime {
        fn from(value: civil::Time) -> Self {
            Self::new(Value::Time(time(value)))
        }
    }

    impl From<civil::DateTime> for DateTime {
        fn from(value: civil::DateTime) -> Self {
            Self::new(Value::DateTime(date(value.date()), time(value.time())))
        }
    }

    /// Uses the date and time of day in the value's time zone.
    impl From<&Zoned> for DateTime {
        fn from(value: &Zoned) -> Self {
            Self::from(value.datetime())
        }
    }

    /// Uses the date and time of day in the value's time zone.
    impl From<Zoned> for DateTime {
        fn from(value: Zoned) -> Self {
            Self::from(&value)
        }
    }

    /// Uses the date and time of day in UTC. Convert the timestamp to a [`Zoned`] to
    /// show another time zone.
    impl From<Timestamp> for DateTime {
        fn from(value: Timestamp) -> Self {
            Self::from(value.to_zoned(TimeZone::UTC))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    fn en(value: DateTime) -> String {
        value.resolve_in(&locale!("en"))
    }

    fn date() -> DateTime {
        DateTime::date(2026, 10, 12).unwrap()
    }

    fn time() -> DateTime {
        DateTime::time(14, 5, 9).unwrap()
    }

    fn date_time() -> DateTime {
        date().at(14, 5, 9).unwrap()
    }

    #[test]
    fn formats_for_the_locale() {
        assert_ne!(en(date()), date().resolve_in(&locale!("de")));
        assert!(en(date()).contains("2026"));
    }

    #[test]
    fn formats_for_the_locale_of_the_context() {
        let cx = Cx::default().with(locale!("de"));
        assert_eq!(date().resolve(&cx), date().resolve_in(&locale!("de")));
    }

    #[test]
    fn shows_only_the_fields_of_the_value() {
        assert!(!en(date()).contains('5'), "{}", en(date()));
        assert!(!en(time()).contains("2026"), "{}", en(time()));
        let both = en(date_time());
        assert!(both.contains("2026") && both.contains("05"), "{both}");
    }

    #[test]
    fn hides_seconds_in_short_times() {
        let short = en(time().time_style(DateTimeStyle::Short));
        let medium = en(time().time_style(DateTimeStyle::Medium));
        assert!(!short.contains("09"), "{short}");
        assert!(medium.contains("09"), "{medium}");
    }

    #[test]
    fn a_style_hides_the_part_without_a_style() {
        let text = en(date_time().date_style(DateTimeStyle::Long));
        assert!(text.contains("2026") && !text.contains("05"), "{text}");
    }

    #[test]
    fn longer_date_styles_show_more() {
        let lengths: Vec<_> = [
            DateTimeStyle::Short,
            DateTimeStyle::Medium,
            DateTimeStyle::Long,
            DateTimeStyle::Full,
        ]
        .into_iter()
        .map(|style| en(date().date_style(style)).len())
        .collect();
        assert!(lengths.is_sorted(), "{lengths:?}");
    }

    #[test]
    fn styles_for_missing_fields_fall_back_to_the_defaults() {
        assert_eq!(en(date().time_style(DateTimeStyle::Full)), en(date()));
    }

    #[test]
    fn uses_the_calendar_of_the_locale() {
        let buddhist = date().resolve_in(&locale!("th-u-ca-buddhist"));
        assert!(buddhist.contains("2569"), "{buddhist}");
    }

    #[test]
    fn rejects_fields_out_of_range() {
        assert!(DateTime::date(2026, 13, 1).is_none());
        assert!(DateTime::time(24, 0, 0).is_none());
    }

    #[cfg(feature = "jiff")]
    #[test]
    fn converts_from_jiff() {
        let civil = jiff::civil::date(2026, 10, 12).at(14, 5, 9, 0);
        assert_eq!(DateTime::from(civil), date_time());
        let timestamp = civil.to_zoned(jiff::tz::TimeZone::UTC).unwrap().timestamp();
        assert_eq!(DateTime::from(timestamp), date_time());
        let zoned = civil
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(2)))
            .unwrap();
        assert_eq!(DateTime::from(&zoned), date_time());
    }
}
