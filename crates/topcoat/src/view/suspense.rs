use crate::{
    Result,
    view::{Child, View, component, emit, live},
};

/// Streams child content in, showing a fallback until it is ready.
///
/// The fallback renders with the surrounding document, so a slow child does
/// not hold the rest of the page back. Once the child content is ready, it
/// replaces the fallback in place.
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
/// The component is a [`live!`] region that emits the fallback and then the
/// child content. Use [`live!`] and [`emit!`] directly for cases it does not
/// cover, like narrating a long-running task through a sequence of emissions.
///
/// [`live!`]: macro@crate::view::live
/// [`emit!`]: macro@crate::view::emit
#[component]
pub async fn suspense(
    /// The view shown until the child content is ready.
    #[into]
    fallback: Child<'_>,
    /// The content that replaces the fallback once it has rendered.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(live! {
        emit! { (fallback) }?;
        emit! { (child) }
    })
}
