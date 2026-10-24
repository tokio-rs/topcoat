#[cfg(feature = "datetime")]
mod datetime;
mod number;

#[cfg(feature = "datetime")]
pub(crate) use datetime::*;
use fluent_bundle::{FluentValue, memoizer::MemoizerKind};

/// Formats Fluent values using the bundle's locale.
///
/// Register this function with
/// [`FluentBundle::set_formatter`](fluent_bundle::bundle::FluentBundle::set_formatter).
/// Number formatting uses Fluent's `NUMBER` options, which follow JavaScript's
/// `Intl.NumberFormat`. Returns `None` for unsupported values so Fluent can format them
/// itself.
pub(crate) fn format_value<M: MemoizerKind>(value: &FluentValue<'_>, intls: &M) -> Option<String> {
    match value {
        FluentValue::Number(number) => number::format_number(number, intls),
        _ => None,
    }
}
