use crate::{
    Error, Result,
    context::{Cx, identity},
    core::identity::SiteKey,
    view::{Child, RegionId, View, component, internal::ErrorBoundaryView},
};

/// Shows a fallback in place of child content that fails to render.
///
/// If any child content fails, the fallback closure receives the error.
/// Its view replaces all of the boundary's content, including content
/// already streamed to the browser. The rest of the page is unchanged.
///
/// Returning `Err` from the closure propagates the error to the enclosing
/// handler. The handler can change the response status only before the
/// response starts streaming.
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
    /// Builds a fallback from the child's error. Return `Err` to propagate
    /// an error instead.
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
