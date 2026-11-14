//! Formatting of money, counts, ratings and dates for the storefront.

use std::fmt::{self, Display, Formatter};

use time::{
    Date, OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description,
};

/// An amount of money in cents, formatted like `$1,234.50`.
pub struct Money(pub u64);

impl Display for Money {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "${}.{:02}", Count(self.0 / 100), self.0 % 100)
    }
}

/// An integer formatted with thousands separators, like `1,204`.
pub struct Count(pub u64);

impl Display for Count {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 < 1000 {
            return write!(f, "{}", self.0);
        }
        write!(f, "{},{:03}", Count(self.0 / 1000), self.0 % 1000)
    }
}

/// A rating with one decimal, like `4.0`.
pub struct Rating(pub f64);

impl Display for Rating {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}

/// A calendar date, like `Mar 4, 2026`.
const SHORT_DATE: &[BorrowedFormatItem<'_>] =
    format_description!("[month repr:short] [day padding:none], [year]");

/// A UTC timestamp, like `Mar 4, 2026 14:05`.
const SHORT_DATE_TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[month repr:short] [day padding:none], [year] [hour]:[minute]");

/// A date in ISO 8601 form, like `2026-03-04`, for `datetime` attributes.
const ISO_DATE: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");

/// A UTC timestamp in ISO 8601 form, like `2026-03-04T14:05Z`, for `datetime` attributes.
const ISO_DATE_TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]Z");

/// Formats a date like `Mar 4, 2026`.
pub fn short_date(date: Date) -> String {
    date.format(SHORT_DATE).expect("dates format without error")
}

/// Formats a UTC timestamp like `Mar 4, 2026 14:05`.
pub fn short_date_time(time: OffsetDateTime) -> String {
    time.format(SHORT_DATE_TIME)
        .expect("timestamps format without error")
}

/// Formats a date like `2026-03-04`.
pub fn iso_date(date: Date) -> String {
    date.format(ISO_DATE).expect("dates format without error")
}

/// Formats a UTC timestamp like `2026-03-04T14:05Z`.
pub fn iso_date_time(time: OffsetDateTime) -> String {
    time.format(ISO_DATE_TIME)
        .expect("timestamps format without error")
}

#[cfg(test)]
mod tests {
    use time::macros::{date, datetime};

    use super::*;

    #[test]
    fn money_groups_thousands_and_pads_cents() {
        assert_eq!(Money(5).to_string(), "$0.05");
        assert_eq!(Money(123_450).to_string(), "$1,234.50");
        assert_eq!(Money(100_000_000).to_string(), "$1,000,000.00");
    }

    #[test]
    fn count_pads_inner_groups() {
        assert_eq!(Count(999).to_string(), "999");
        assert_eq!(Count(1_004).to_string(), "1,004");
        assert_eq!(Count(12_000_070).to_string(), "12,000,070");
    }

    #[test]
    fn dates_drop_leading_zeros_from_days_only() {
        assert_eq!(short_date(date!(2026 - 03 - 04)), "Mar 4, 2026");
        assert_eq!(
            short_date_time(datetime!(2026-03-04 04:05 UTC)),
            "Mar 4, 2026 04:05"
        );
    }

    #[test]
    fn iso_forms_pad_every_field() {
        assert_eq!(iso_date(date!(2026 - 03 - 04)), "2026-03-04");
        assert_eq!(
            iso_date_time(datetime!(2026-03-04 04:05:59 UTC)),
            "2026-03-04T04:05Z"
        );
    }
}
