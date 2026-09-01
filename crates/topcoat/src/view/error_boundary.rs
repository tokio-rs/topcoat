use crate::{
    Error, Result,
    view::{Child, View, component, emit, live},
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
/// The component is a [`live!`] region that emits the child content and
/// matches on the result. Use [`live!`] and [`emit!`] directly for cases it
/// does not cover, like retrying the child after an error.
///
/// [`live!`]: macro@crate::view::live
/// [`emit!`]: macro@crate::view::emit
#[component]
pub async fn error_boundary<V, F>(
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
    Ok(live! {
        match emit! { (child) } {
            Err(error) => {
                let fallback = Child::new(fallback(error)?);
                emit! { (fallback) }
            }
            emitted => emitted,
        }
    })
}
