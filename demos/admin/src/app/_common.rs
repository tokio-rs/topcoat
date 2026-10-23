//! Helpers shared by several pages. The `_common` group has no routes.

use topcoat::{
    context::Cx,
    l10n::{Translation, fluent::messages},
    router::{
        error::{SeeOther, see_other},
        href,
    },
};

use crate::features::{catalog::Money, user::Role};

messages!("{mod}/l10n/{locale}.ftl");

/// Formats an amount in the request's language, such as `$1,234.50`.
pub fn money(amount: Money) -> Translation<'static> {
    t!(money, amount = amount.dollars())
}

pub fn role_label(role: Role) -> Translation<'static> {
    match role {
        Role::Admin => t!(role-admin),
        Role::Staff => t!(role-staff),
        Role::Viewer => t!(role-viewer),
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
