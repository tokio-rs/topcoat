//! Simulated sales and visitor counts, calculated from the current time.
//! They rise during the day and fall at night. Everyone sees the same values
//! at a given time; these figures do not use the database.

use std::hash::{DefaultHasher, Hash, Hasher};

use jiff::Timestamp;

use crate::features::catalog::Money;

const MINUTE: i64 = 60;

const DAY: i64 = 24 * 60 * MINUTE;

/// Peak activity, measured in seconds since midnight UTC.
const PEAK: i64 = 16 * 60 * MINUTE;

/// Sales history length in minutes.
const SALES_HISTORY: i64 = 30;

/// Visitor history length in seconds.
const VISITOR_HISTORY: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalesStats {
    /// Sales since midnight UTC.
    pub today: Sales,
    /// Yesterday's sales from midnight UTC through the same time of day.
    pub yesterday: Sales,
    /// Sales per minute, oldest first. The last entry is the current, incomplete minute.
    pub by_minute: Vec<Sales>,
}

impl SalesStats {
    pub fn now() -> Self {
        Self::at(Timestamp::now().as_second())
    }

    /// Calculates sales at a Unix timestamp in seconds.
    fn at(second: i64) -> Self {
        let midnight = second - second.rem_euclid(DAY);
        let current_minute = second - second.rem_euclid(MINUTE);

        Self {
            today: Sales::between(midnight, second + 1),
            yesterday: Sales::between(midnight - DAY, second - DAY + 1),
            by_minute: (0..SALES_HISTORY)
                .rev()
                .map(|ago| {
                    let start = current_minute - ago * MINUTE;
                    Sales::between(start, (start + MINUTE).min(second + 1))
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisitorStats {
    pub current: i64,
    /// Visitor count at this time yesterday.
    pub yesterday: i64,
    /// Visitor counts per second, oldest first.
    pub by_second: Vec<i64>,
}

impl VisitorStats {
    pub fn now() -> Self {
        Self::at(Timestamp::now().as_second())
    }

    /// Calculates visitor counts at a Unix timestamp in seconds.
    fn at(second: i64) -> Self {
        Self {
            current: visitors(second),
            yesterday: visitors(second - DAY),
            by_second: (0..VISITOR_HISTORY)
                .rev()
                .map(|ago| visitors(second - ago))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sales {
    pub orders: i64,
    pub revenue: Money,
}

impl Sales {
    /// Calculates sales for `start..end`, using Unix timestamps in seconds.
    fn between(start: i64, end: i64) -> Self {
        let mut sales = Self::default();
        let mut minute = start.div_euclid(MINUTE);
        while minute * MINUTE < end {
            let count = orders_in_minute(minute);
            for order in 0..count {
                let spacing = MINUTE / count;
                let placed =
                    minute * MINUTE + order * spacing + noise(("placed", minute, order), spacing);
                if (start..end).contains(&placed) {
                    sales.orders += 1;
                    sales.revenue.0 += 2_500 + noise(("total", minute, order), 15_000);
                }
            }
            minute += 1;
        }
        sales
    }

    pub fn average_order(&self) -> Money {
        if self.orders == 0 {
            Money(0)
        } else {
            Money(self.revenue.0 / self.orders)
        }
    }
}

/// Daily activity factor, ranging from 350 at night to 1000 at the peak.
fn busyness(second: i64) -> i64 {
    let from_peak = (second.rem_euclid(DAY) - PEAK).abs();
    let from_peak = from_peak.min(DAY - from_peak);
    1_000 - from_peak * 650 / (DAY / 2)
}

/// Simulated orders for a minute, counted from the Unix epoch.
fn orders_in_minute(minute: i64) -> i64 {
    let expected = busyness(minute * MINUTE) * 14 / 1_000;
    (expected + noise(("orders", minute), 5) - 2).max(0)
}

fn visitors(second: i64) -> i64 {
    let expected = busyness(second) * 180 / 1_000;
    expected + wave(second, 30, 40) + wave(second, 7, 12) + noise(("visitors", second), 5) - 2
}

/// Interpolates between random values, choosing a new target every `period`
/// seconds. Centers the result around zero with a range of about `height`.
fn wave(second: i64, period: i64, height: i64) -> i64 {
    let step = second.div_euclid(period);
    let from = noise(("wave", period, step), height);
    let to = noise(("wave", period, step + 1), height);
    from + (to - from) * second.rem_euclid(period) / period - height / 2
}

/// Produces a repeatable value in `0..below` from `seed`.
fn noise(seed: impl Hash, below: i64) -> i64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let value = hasher.finish() % below.unsigned_abs();
    i64::try_from(value).expect("a number below an `i64` fits in an `i64`")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIDNIGHT: i64 = 20_000 * DAY;

    #[test]
    fn sales_only_grow_during_a_day() {
        let mut previous = SalesStats::at(MIDNIGHT).today;
        for second in (MIDNIGHT..MIDNIGHT + DAY).step_by(599) {
            let today = SalesStats::at(second).today;
            assert!(today.orders >= previous.orders, "orders shrank at {second}");
            assert!(
                today.revenue >= previous.revenue,
                "revenue shrank at {second}"
            );
            previous = today;
        }
        assert!(previous.orders > 0);
    }

    #[test]
    fn starts_over_at_midnight() {
        let before = SalesStats::at(MIDNIGHT - 1).today;
        let after = SalesStats::at(MIDNIGHT).today;
        assert!(after.orders <= 1);
        assert!(after.revenue < before.revenue);
    }

    #[test]
    fn minutes_add_up_to_the_day() {
        let second = MIDNIGHT + 3 * 60 * MINUTE + 17;
        let stats = SalesStats::at(second);
        let recent = stats
            .by_minute
            .iter()
            .map(|sales| sales.orders)
            .sum::<i64>();
        let earlier = Sales::between(
            MIDNIGHT,
            second - second.rem_euclid(MINUTE) - (SALES_HISTORY - 1) * MINUTE,
        );
        assert_eq!(earlier.orders + recent, stats.today.orders);
    }

    #[test]
    fn always_has_visitors() {
        for second in (MIDNIGHT..MIDNIGHT + DAY).step_by(13) {
            assert!(visitors(second) > 0, "no visitors at {second}");
        }
    }
}
