mod number;

use fluent_bundle::{FluentValue, memoizer::MemoizerKind};

/// Formats the values of Fluent messages for the bundle's locale.
///
/// Install it with
/// [`FluentBundle::set_formatter`](fluent_bundle::bundle::FluentBundle::set_formatter).
/// Numbers follow the options of Fluent's `NUMBER` function, which match
/// those of JavaScript's `Intl.NumberFormat`. Returns `None` for values it
/// does not format, which leaves them to Fluent.
pub fn format_value<M: MemoizerKind>(value: &FluentValue<'_>, intls: &M) -> Option<String> {
    match value {
        FluentValue::Number(number) => number::format_number(number, intls),
        _ => None,
    }
}
