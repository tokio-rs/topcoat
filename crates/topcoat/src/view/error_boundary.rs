use crate::{
    Error, Result,
    context::{Cx, identity},
    core::identity::SiteKey,
    view::{Child, RegionId, View, component, internal::ErrorBoundaryView},
};

/// Shows a fallback in place of child content that fails to render.
///
/// The child renders as if it stood in the boundary's place. When any part
/// of it returns an error, the error is passed to the fallback closure and
/// the view it returns replaces the boundary's content, leaving the rest of
/// the page intact. Content the child already streamed out is replaced along
/// with it.
///
/// Returning `Err` from the closure rethrows: the error propagates as if
/// there were no boundary, so an error the fallback does not handle can
/// still reach the enclosing handler and set the response status.
///
/// ```rust
/// use topcoat::{
///     Result,
///     view::{View, component, error_boundary, view},
/// };
///
/// #[component]
/// async fn stats() -> Result<impl View> {
///     let visits = fetch_visits().await?;
///     Ok(view! { <p>(visits) " visits"</p> })
/// }
/// # async fn fetch_visits() -> Result<u32> { Ok(3) }
///
/// #[component]
/// async fn dashboard() -> Result<impl View> {
///     Ok(view! {
///         error_boundary(
///             fallback: |error| Ok(view! {
///                 <p>"The stats are unavailable: " (error.to_string())</p>
///             }),
///             stats()
///         )
///     })
/// }
/// ```
///
/// Use [`live!`] and [`emit!`] directly for cases the component does not
/// cover, like retrying the child after an error.
///
/// [`live!`]: macro@crate::view::live
/// [`emit!`]: macro@crate::view::emit
#[component]
pub async fn error_boundary<V, F>(
    cx: &Cx,
    /// Builds the view shown when the child content fails, from the error
    /// that caused it. Returns the error itself, or another one, to rethrow.
    fallback: F,
    /// The content the boundary guards.
    #[default]
    child: Child<'_>,
) -> Result<impl View>
where
    V: View,
    F: FnOnce(Error) -> Result<V> + Send,
{
    const SITE: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    let region = RegionId::new(identity(cx), SITE);
    Ok(ErrorBoundaryView::new(region, fallback, child))
}
