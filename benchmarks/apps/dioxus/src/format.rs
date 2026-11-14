//! Display wrappers that format values for the storefront.

use std::fmt::{self, Display, Formatter};

use time::{Date, OffsetDateTime, format_description::BorrowedFormatItem, macros::format_description};

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

/// Formats a calendar date like `Mar 4, 2026`.
pub fn short_date(date: Date) -> String {
    format(date.format(SHORT_DATE))
}

/// Formats a calendar date in ISO 8601 form, like `2026-03-04`, for `datetime` attributes.
pub fn iso_date(date: Date) -> String {
    format(date.format(ISO_DATE))
}

/// Formats a UTC timestamp like `Mar 4, 2026 14:05`.
pub fn short_date_time(time: OffsetDateTime) -> String {
    format(time.format(SHORT_DATE_TIME))
}

/// Formats a UTC timestamp in ISO 8601 form without seconds, like `2026-03-04T14:05Z`, for
/// `datetime` attributes.
pub fn iso_date_time(time: OffsetDateTime) -> String {
    format(time.format(ISO_DATE_TIME))
}

const SHORT_DATE: &[BorrowedFormatItem<'_>] =
    format_description!("[month repr:short] [day padding:none], [year]");
const ISO_DATE: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");
const SHORT_DATE_TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[month repr:short] [day padding:none], [year] [hour]:[minute]");
const ISO_DATE_TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]Z");

/// Unwraps a formatting result. The descriptions above only use components every `Date` and
/// `OffsetDateTime` has, so formatting cannot fail.
fn format(result: Result<String, time::error::Format>) -> String {
    result.expect("the date format only uses available components")
}

#[cfg(test)]
mod tests {
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
        let time = time::macros::datetime!(2026-03-04 04:05:09 UTC);
        assert_eq!(short_date(time.date()), "Mar 4, 2026");
        assert_eq!(iso_date(time.date()), "2026-03-04");
        assert_eq!(short_date_time(time), "Mar 4, 2026 04:05");
        assert_eq!(iso_date_time(time), "2026-03-04T04:05Z");
    }
}
