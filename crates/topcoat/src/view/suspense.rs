use crate::{
    Result,
    context::{Cx, identity},
    core::identity::SiteKey,
    view::{Child, RegionId, View, component, internal::SuspenseView},
};

/// Streams child content in, showing a fallback until it is ready.
///
/// The child is given the first chance to render. When its content is ready
/// right away, it renders in place and the fallback never shows. Otherwise
/// the fallback renders with the surrounding document, so a slow child does
/// not hold the rest of the page back, and the child content replaces the
/// fallback in place once it is ready.
///
/// Errors from the child are not caught; wrap the child in an
/// [`error_boundary`](super::error_boundary) to handle them.
///
/// ```rust
/// use topcoat::{
///     Result,
///     view::{View, component, suspense, view},
/// };
///
/// #[component]
/// async fn quote() -> Result<impl View> {
///     let quote = fetch_quote().await;
///     Ok(view! { <blockquote>(quote)</blockquote> })
/// }
/// # async fn fetch_quote() -> &'static str { "..." }
///
/// #[component]
/// async fn page() -> Result<impl View> {
///     Ok(view! {
///         suspense(
///             fallback: view! { <p>"Loading..."</p> },
///             quote()
///         )
///     })
/// }
/// ```
///
/// Use [`live!`] and [`emit!`] directly for cases the component does not
/// cover, like narrating a long-running task through a sequence of
/// emissions.
///
/// [`live!`]: macro@crate::view::live
/// [`emit!`]: macro@crate::view::emit
#[component]
pub async fn suspense(
    cx: &Cx,
    /// The view shown until the child content is ready.
    #[into]
    fallback: Child<'_>,
    /// The content that replaces the fallback once it has rendered.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    const SITE: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    let region = RegionId::new(identity(cx), SITE);
    Ok(SuspenseView::new(region, fallback, child))
}
