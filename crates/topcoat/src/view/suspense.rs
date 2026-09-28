// Links to the router extension become plain text without the router.
#![cfg_attr(not(feature = "router"), allow(rustdoc::broken_intra_doc_links))]

use crate::{
    Result,
    context::{Cx, identity, try_app_context, try_request_context},
    core::identity::SiteKey,
    view::{Child, RegionId, View, component, internal::SuspenseView},
};

/// Shows a fallback while child content loads, then streams the content in.
///
/// If the child is ready immediately, the fallback never appears. Otherwise,
/// the fallback renders with the page and the child replaces it when ready.
///
/// Use `mode: SuspenseMode::Wait` to wait for the child's first content
/// instead of showing the fallback. If you omit `mode`, [`suspense_mode`]
/// checks the context and uses [`SuspenseMode::Stream`] if no mode is set.
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
/// Use [`live!`] and [`emit!`] to control a sequence of updates directly.
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
    /// Whether to show a fallback while loading or wait for the first content.
    /// Uses [`suspense_mode`] if omitted.
    #[into]
    #[default]
    mode: Option<SuspenseMode>,
) -> Result<impl View> {
    const SITE: SiteKey = SiteKey::new(file!(), line!(), column!(), 0);
    let region = RegionId::new(identity(cx), SITE);
    let mode = mode.unwrap_or_else(|| suspense_mode(cx));
    let wait = mode == SuspenseMode::Wait;
    Ok(SuspenseView::new(region, fallback, child, wait))
}

/// Whether a [`suspense`] boundary shows a fallback while its child loads.
///
/// The default, [`Stream`](Self::Stream), shows the fallback while the child
/// loads, then replaces it with the child's content. [`Wait`](Self::Wait)
/// delays rendering until the child's initial content is ready and never
/// shows the fallback. Use `Wait` when that content needs to be available
/// without JavaScript. The child can still send live updates afterward.
///
/// Pass `mode` to set the behavior for one boundary:
///
/// ```rust
/// use topcoat::{
///     Result,
///     view::{SuspenseMode, View, component, suspense, view},
/// };
///
/// #[component]
/// async fn product() -> Result<impl View> {
///     let description = fetch_description().await;
///     Ok(view! { <p>(description)</p> })
/// }
/// # async fn fetch_description() -> &'static str { "..." }
///
/// #[component]
/// async fn page() -> Result<impl View> {
///     Ok(view! {
///         suspense(
///             fallback: view! { <p>"Loading..."</p> },
///             mode: SuspenseMode::Wait,
///             product()
///         )
///     })
/// }
/// ```
///
/// Set the router's default with [`RouterSuspenseExt::suspense`]. To use a
/// different mode for a request, add a `SuspenseMode` to its context. See
/// [`suspense_mode`] for how the default is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SuspenseMode {
    /// Shows the fallback if the child's initial content is not ready, then
    /// replaces it when the content is ready. This is the default.
    #[default]
    Stream,
    /// Waits for the child's initial content without showing the fallback.
    Wait,
}

/// Returns the default mode for [`suspense`] in this context.
///
/// Checks these sources in order and uses the first value it finds:
///
/// 1. A `SuspenseMode` in the request context.
/// 2. A `SuspenseMode` in the app context.
/// 3. [`SuspenseMode::Stream`].
#[must_use]
pub fn suspense_mode(cx: &Cx) -> SuspenseMode {
    try_request_context::<SuspenseMode>(cx)
        .or_else(|| try_app_context::<SuspenseMode>(cx))
        .copied()
        .unwrap_or_default()
}

/// Configures the default suspense mode on a [`RouterBuilder`](crate::router::RouterBuilder).
#[cfg(feature = "router")]
pub trait RouterSuspenseExt {
    /// Sets the default mode for the router's [`suspense`] boundaries.
    ///
    /// Stores `mode` in the app context. A boundary's `mode` argument or a
    /// `SuspenseMode` in the request context overrides this default.
    ///
    /// ```rust
    /// use topcoat::{
    ///     router::Router,
    ///     view::{RouterSuspenseExt, SuspenseMode},
    /// };
    ///
    /// let router = Router::builder().suspense(SuspenseMode::Wait).build();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if a `SuspenseMode` is already registered in the app context.
    #[must_use]
    #[track_caller]
    fn suspense(self, mode: SuspenseMode) -> Self;
}

#[cfg(feature = "router")]
impl RouterSuspenseExt for crate::router::RouterBuilder {
    #[track_caller]
    fn suspense(self, mode: SuspenseMode) -> Self {
        self.app_context(mode)
    }
}

#[cfg(all(test, feature = "router"))]
mod tests {
    use super::*;
    use crate::{
        router::{Body, Router, RouterBuilder, page, request::Request, to_bytes},
        view::view,
    };

    /// Yields once before rendering its content.
    #[component]
    async fn deferred() -> Result<impl View> {
        tokio::task::yield_now().await;
        Ok(view! { <p>"content"</p> })
    }

    #[page("/")]
    async fn home() -> Result<impl View> {
        Ok(view! { suspense(fallback: view! { <p>"loading"</p> }, deferred()) })
    }

    /// Sends a `GET /` request to `router` and collects the response body.
    async fn fetch(router: Router) -> String {
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = router.handle(request).await;
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn a_router_streams_suspense_by_default() {
        let html = fetch(RouterBuilder::new().page(home).build()).await;
        assert!(html.contains("<p>loading</p>"), "{html}");
        assert!(html.contains("<p>content</p>"), "{html}");
    }

    #[tokio::test]
    async fn a_waiting_router_sends_suspense_content_settled() {
        let router = RouterBuilder::new()
            .suspense(SuspenseMode::Wait)
            .page(home)
            .build();
        let html = fetch(router).await;
        assert!(!html.contains("loading"), "{html}");
        assert!(!html.contains("topcoat::region"), "{html}");
        assert!(html.contains("<p>content</p>"), "{html}");
    }
}
