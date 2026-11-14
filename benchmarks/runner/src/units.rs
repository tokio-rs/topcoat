//! Durations and human readable numbers.

use std::{fmt, str::FromStr, time::Duration};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A duration written as `500ms`, `10s`, `2m` or `1h`. A bare number means seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dur(pub Duration);

impl Dur {
    /// Formats the duration for `oha -z`, which accepts `ms` and `s` suffixes.
    pub fn oha_arg(self) -> String {
        format!("{}ms", self.0.as_millis().max(1))
    }
}

impl FromStr for Dur {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let split = text
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(text.len());
        let (number, unit) = text.split_at(split);
        let value: f64 = number.parse().map_err(|_| {
            format!("invalid duration `{text}`, expected e.g. `500ms`, `10s` or `2m`")
        })?;
        let factor = match unit.trim() {
            "ms" => 0.001,
            "" | "s" => 1.0,
            "m" | "min" => 60.0,
            "h" => 3600.0,
            other => return Err(format!("unknown duration unit `{other}` in `{text}`")),
        };
        if !value.is_finite() || value < 0.0 {
            return Err(format!("invalid duration `{text}`"));
        }
        Ok(Self(Duration::from_secs_f64(value * factor)))
    }
}

impl fmt::Display for Dur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let millis = self.0.as_millis();
        if !millis.is_multiple_of(1000) {
            write!(f, "{millis}ms")
        } else if millis >= 60_000 && millis.is_multiple_of(60_000) {
            write!(f, "{}m", millis / 60_000)
        } else {
            write!(f, "{}s", millis / 1000)
        }
    }
}

impl Serialize for Dur {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Dur {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Number(f64),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Text(text) => text.parse().map_err(serde::de::Error::custom),
            Raw::Number(secs) if secs.is_finite() && secs >= 0.0 => {
                Ok(Self(Duration::from_secs_f64(secs)))
            }
            Raw::Number(secs) => Err(serde::de::Error::custom(format!("invalid duration {secs}"))),
        }
    }
}

/// Formats a byte count with binary units, e.g. `12.3 MiB`.
#[expect(clippy::cast_precision_loss, reason = "display only")]
pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut amount = value as f64;
    let mut unit = 0;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{amount:.1} {}", UNITS[unit])
    }
}

/// Formats a count with thousands separators, e.g. `12,345`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "display only"
)]
pub fn count(value: f64) -> String {
    let rounded = value.round().max(0.0) as u64;
    let digits = rounded.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Formats milliseconds with a precision that suits the magnitude.
pub fn millis(value: f64) -> String {
    if value < 1.0 {
        format!("{value:.3} ms")
    } else if value < 100.0 {
        format!("{value:.2} ms")
    } else {
        format!("{value:.0} ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_round_trip_through_text() {
        for text in ["500ms", "10s", "2m", "1500ms"] {
            let parsed: Dur = text.parse().unwrap();
            assert_eq!(parsed.to_string().parse::<Dur>().unwrap(), parsed);
        }
        assert_eq!("1.5s".parse::<Dur>().unwrap(), "1500ms".parse().unwrap());
        assert_eq!("90".parse::<Dur>().unwrap(), "90s".parse().unwrap());
        assert!("10 parsecs".parse::<Dur>().is_err());
    }

    #[test]
    fn counts_group_thousands() {
        assert_eq!(count(1_234_567.0), "1,234,567");
        assert_eq!(count(999.0), "999");
    }
}
