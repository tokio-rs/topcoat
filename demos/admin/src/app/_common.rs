//! Helpers shared by several pages. The `_common` group has no routes.

use jiff::Timestamp;
use topcoat::{
    context::Cx,
    l10n::{Number, Translation, fluent::messages},
    router::{
        error::{SeeOther, see_other},
        href,
    },
};

use crate::features::{catalog::Money, user::Role};

messages!("{mod}/l10n/{locale}.ftl");

/// Formats an amount in the request's language, such as `$1,234.50`.
pub fn money(amount: Money) -> Translation<'static> {
    t!(money, amount: amount.dollars())
}

/// Formats an amount for a form field, such as `1234.50` in English. It has no currency
/// symbol or thousands separators, so [`Money`] parses the result.
pub fn money_input(amount: Money) -> Number {
    Number::new(amount.dollars())
        .min_fraction_digits(2)
        .max_fraction_digits(2)
        .grouping(false)
}

/// Formats the date and time of day of `at` in UTC, such as `Oct 12, 2026, 2:05 PM UTC`.
pub fn timestamp(at: Timestamp) -> Translation<'static> {
    t!(timestamp, at)
}

pub fn role_label(role: Role) -> Translation<'static> {
    match role {
        Role::Admin => t!(role_admin),
        Role::Staff => t!(role_staff),
        Role::Viewer => t!(role_viewer),
    }
}

/// Redirects back to the page a form was submitted from.
///
/// Only redirects to local paths because the submitted path is untrusted.
/// Redirects to the dashboard instead of other paths.
pub fn see_back(cx: &Cx, path: &str) -> SeeOther {
    let local = path.starts_with('/') && !path.starts_with("//") && !path.contains('\\');
    if local {
        see_other(path)
    } else {
        see_other(href!(super::_admin::_dashboard::page).resolve(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::locale::LOCALES;

    #[test]
    fn money_input_reads_back_in_every_language() {
        for locale in &LOCALES {
            for cents in [5, 2_499, 123_456_789] {
                let text = money_input(Money(cents)).resolve_in(locale);
                assert_eq!(text.parse(), Ok(Money(cents)), "{locale}: {text}");
            }
        }
    }
}
