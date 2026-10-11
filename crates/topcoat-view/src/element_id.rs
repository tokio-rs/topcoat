use std::panic::Location;

use topcoat_core::{
    context::{Cx, identity},
    identity::SiteKey,
};

/// Returns an HTML `id` that is unique to this call and stable across renders.
///
/// The id comes from the identity of `cx` and the location of this call, so each
/// component instance gets its own ids. Use it to connect elements that refer to each
/// other by id, such as a label and its input or a button and the popover it opens:
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
///     let id = element_id(cx);
///     Ok(view! {
///         <label for=(&id)>"Email"</label>
///         <input id=(&id) type="email" name="email">
///     })
/// }
/// ```
///
/// Calls at different locations return different ids. A call that runs repeatedly,
/// such as one in a helper invoked from a loop, returns the same id each time. Pass a
/// context derived with [`Cx::keyed`] to tell those calls apart. The id starts with a
/// letter and contains only ASCII letters, digits, `-` and `_`, so it also works
/// unescaped in CSS selectors.
///
/// # Panics
///
/// Panics if `cx` belongs to a memoized call or carries an ambiguous identity, such as
/// one inside a `view!` loop without a `#[key]`.
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

    /// Returns the id of a single call site, however often it runs.
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
