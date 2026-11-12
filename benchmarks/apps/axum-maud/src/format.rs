//! Display wrappers that format values for the storefront.
//!
//! Dates use the `time` crate's formatter. Money and counts are formatted by hand because the
//! standard library has no locale-aware number formatting.

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
pub struct ShortDate(pub Date);

impl Display for ShortDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const FORMAT: &[BorrowedFormatItem<'_>] =
            format_description!("[month repr:short] [day padding:none], [year]");
        f.write_str(&self.0.format(FORMAT).map_err(|_| fmt::Error)?)
    }
}

/// A UTC timestamp, like `Mar 4, 2026 14:05`.
pub struct ShortDateTime(pub OffsetDateTime);

impl Display for ShortDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const FORMAT: &[BorrowedFormatItem<'_>] =
            format_description!("[month repr:short] [day padding:none], [year] [hour]:[minute]");
        f.write_str(&self.0.format(FORMAT).map_err(|_| fmt::Error)?)
    }
}

/// A date in ISO 8601 form, like `2026-03-04`, for `datetime` attributes.
pub struct IsoDate(pub Date);

impl Display for IsoDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const FORMAT: &[BorrowedFormatItem<'_>] = format_description!("[year]-[month]-[day]");
        f.write_str(&self.0.format(FORMAT).map_err(|_| fmt::Error)?)
    }
}

/// A UTC timestamp in ISO 8601 form, like `2026-03-04T14:05Z`, for `datetime` attributes.
pub struct IsoDateTime(pub OffsetDateTime);

impl Display for IsoDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const FORMAT: &[BorrowedFormatItem<'_>] =
            format_description!("[year]-[month]-[day]T[hour]:[minute]Z");
        f.write_str(&self.0.format(FORMAT).map_err(|_| fmt::Error)?)
    }
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
        assert_eq!(ShortDate(date!(2026 - 03 - 04)).to_string(), "Mar 4, 2026");
        assert_eq!(
            ShortDateTime(datetime!(2026-03-04 04:05:59 UTC)).to_string(),
            "Mar 4, 2026 04:05"
        );
    }

    #[test]
    fn iso_forms_pad_every_field() {
        assert_eq!(IsoDate(date!(2026 - 03 - 04)).to_string(), "2026-03-04");
        assert_eq!(
            IsoDateTime(datetime!(2026-03-04 04:05:59 UTC)).to_string(),
            "2026-03-04T04:05Z"
        );
    }
}
