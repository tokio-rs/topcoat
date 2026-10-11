use std::panic::Location;

use topcoat_core::{
    context::{Cx, identity},
    identity::SiteKey,
};

/// Creates an HTML ID that stays the same when the component renders again.
///
/// The ID depends on the identity in `cx` and where you call this function. Each
/// component instance gets its own IDs. Use them to connect a label to an input or a
/// button to a popover:
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     view::{View, component, element_id, view},
/// };
///
/// #[component]
/// async fn email_field(cx: &Cx) -> Result<impl View> {
///     Ok(view! {
///         let id = element_id(cx);
///         <label for=(&id)>"Email"</label>
///         <input id=(&id) type="email" name="email">
///     })
/// }
/// ```
///
/// Calls on different lines or columns produce different IDs. Repeating a call at the
/// same location with the same context produces the same ID. If a loop calls a helper
/// that creates IDs, pass a different [`Cx::keyed`] context for each item.
///
/// IDs start with a letter and contain only ASCII letters, digits, `-`, and `_`. You
/// can use them in CSS selectors without escaping them.
///
/// # Panics
///
/// Panics inside a memoized call or when `cx` cannot identify a single instance. This
/// happens, for example, when a `view!` loop renders the component without a `#[key]`.
#[must_use]
#[track_caller]
pub fn element_id(cx: &Cx) -> String {
    let identity = identity(cx).child(SiteKey::from_location(Location::caller()));
    format!("tc-{identity}")
}

#[cfg(test)]
mod tests {
    use topcoat_core::{context::with_identity, identity::Identity};

    use super::*;

    /// Calls `element_id` from the same source location each time.
    fn id_at_one_site(cx: &Cx) -> String {
        element_id(cx)
    }

    #[test]
    fn repeated_calls_at_one_site_return_the_same_id() {
        let cx = Cx::default();
        assert_eq!(id_at_one_site(&cx), id_at_one_site(&cx.clone()));
    }

    #[test]
    fn different_sites_return_different_ids() {
        let cx = Cx::default();
        assert_ne!(element_id(&cx), element_id(&cx));
    }

    #[test]
    fn keyed_contexts_return_different_ids() {
        let cx = Cx::default();
        let ids: Vec<_> = (0..3).map(|key| id_at_one_site(&cx.keyed(key))).collect();
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert_ne!(ids[0], ids[2]);
    }

    #[test]
    fn ids_are_valid_css_identifiers() {
        let cx = Cx::default();
        for key in 0..64 {
            let id = id_at_one_site(&cx.keyed(key));
            assert!(id.starts_with(|c: char| c.is_ascii_alphabetic()), "{id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "{id}",
            );
        }
    }

    #[test]
    #[should_panic(expected = "unkeyed-test-loop")]
    fn an_ambiguous_identity_panics() {
        let site = SiteKey::new(file!(), line!(), column!(), 0);
        let cx = with_identity(
            Cx::default(),
            Identity::ROOT.ambiguous_child(site, "unkeyed-test-loop"),
        );
        let _ = element_id(&cx);
    }
}
