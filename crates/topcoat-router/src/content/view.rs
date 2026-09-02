use std::{
    fmt::Write,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::future::poll_fn;
use http::{HeaderMap, StatusCode};
use http_body::Frame;
use pin_project_lite::pin_project;
use topcoat_core::{context::Cx, error::Result};
use topcoat_view::{BoxView, Formatter, View, ViewExt, ViewHandle, internal::MoveView};

use crate::{
    Body, BoxError,
    content::Html,
    error::RedirectError,
    response::{AsyncIntoResponse, IntoResponse, Response},
};

impl IntoResponse for ViewHandle {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let rendered = self.render_response(cx);
        html_response(cx, rendered.html, rendered.status_code, rendered.headers)
    }
}

impl AsyncIntoResponse for BoxView<'static> {
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self, cx)
    }
}

impl<Fut> AsyncIntoResponse for MoveView<Fut>
where
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn async_into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        stream(self.boxed(), cx)
    }
}

async fn stream<V: View + Unpin + 'static>(mut view: V, cx: &Cx) -> Result<Response> {
    let mut pinned_view = Pin::new(&mut view);
    let first = poll_fn(|cx| pinned_view.as_mut().poll_first(cx)).await?;
    let rendered = first.content.render_response(cx);
    if first.live {
        let body = ViewBody {
            cx: cx.clone(),
            first: Some(rendered.html),
            script: Some(SWAP_SCRIPT),
            done: false,
            view,
        };
        html_response(cx, Body::new(body), rendered.status_code, rendered.headers)
    } else {
        html_response(
            cx,
            Body::new(rendered.html),
            rendered.status_code,
            rendered.headers,
        )
    }
}

/// Builds an HTML response around `body`, applying the status code and
/// headers a view declared.
fn html_response(
    cx: &Cx,
    body: impl Into<Body>,
    status_code: Option<StatusCode>,
    headers: HeaderMap,
) -> Result<Response> {
    let mut response = Html(body.into()).into_response(cx)?;
    if let Some(status_code) = status_code {
        *response.status_mut() = status_code;
    }
    response.headers_mut().extend(headers);
    Ok(response)
}

/// Applies streamed swaps in the browser: replaces the content between a
/// region's marker comments with the template the swap arrived in.
///
/// Sent once per streaming response, ahead of the first swap. The `??=`
/// guard steps aside for an applier the page installed itself.
const SWAP_SCRIPT: &str = r"<script>
window.topcoat ??= {
    swap(id) {
        const script = document.currentScript;
        const template = script.previousElementSibling;
        let open = null;
        let close = null;
        const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_COMMENT);
        while (walker.nextNode()) {
            const comment = walker.currentNode;
            if (comment.data === `topcoat::region::start(${id})`) open = comment;
            else if (comment.data === `topcoat::region::end(${id})`) close = comment;
        }
        if (open && close) {
            while (open.nextSibling && open.nextSibling !== close) open.nextSibling.remove();
            close.parentNode.insertBefore(template.content, close);
        }
        template.remove();
        script.remove();
    },
};
</script>";

/// Builds the script a mid-stream redirect is sent as: a navigation to the
/// redirect's target. `replace` keeps the partially streamed page out of the
/// session history, so going back skips it.
fn redirect_script(redirect: &RedirectError) -> String {
    // The location was built from a `str`, so it converts back.
    let uri = redirect.location().to_str().unwrap_or_default();
    let mut location = String::with_capacity(uri.len());
    for c in uri.chars() {
        match c {
            '\\' => location.push_str("\\\\"),
            '"' => location.push_str("\\\""),
            // Keeps the target from closing the script element early.
            '<' => location.push_str("\\x3C"),
            c => location.push(c),
        }
    }
    format!("<script>window.location.replace(\"{location}\")</script>")
}

pin_project! {
    struct ViewBody<V> {
        cx: Cx,
        first: Option<String>,
        // The swap applier, taken by the first swap it is sent ahead of.
        script: Option<&'static str>,
        // Whether the view has reported it has no further swaps. Polling a
        // view past that point resumes a future that already completed.
        done: bool,
        #[pin]
        view: V,
    }
}

impl<V: View + 'static> http_body::Body for ViewBody<V> {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        if let Some(first) = this.first.take() {
            return Poll::Ready(Some(Ok(Frame::data(first.into()))));
        }
        if *this.done {
            return Poll::Ready(None);
        }
        match this.view.poll_swap(cx) {
            Poll::Ready(Ok(Some(swap))) => {
                let script = this.script.take();
                let region = swap.region;
                // The envelope's fixed parts and two region ids on top of
                // the replacement's own estimate.
                let mut envelope = String::with_capacity(
                    script.map_or(0, str::len) + swap.replacement.size_hint() + 96,
                );
                {
                    let mut f = Formatter::new(&mut envelope);
                    if let Some(script) = script {
                        f.write_str(script);
                    }
                    write!(f, "<template data-topcoat-swap=\"{region}\">").unwrap();
                    swap.replacement.render_into(this.cx, &mut f);
                    write!(f, "</template><script>topcoat.swap({region})</script>").unwrap();
                }
                Poll::Ready(Some(Ok(Frame::data(envelope.into()))))
            }
            Poll::Ready(Ok(None)) => {
                *this.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(error)) => {
                *this.done = true;
                // The response committed with the first content, so a
                // redirect can no longer change the status line; it degrades
                // to a client-side navigation instead.
                match error.downcast::<RedirectError>() {
                    Ok(redirect) => {
                        Poll::Ready(Some(Ok(Frame::data(redirect_script(&redirect).into()))))
                    }
                    Err(error) => Poll::Ready(Some(Err(error.into()))),
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.first.is_none() && self.done
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use futures_util::StreamExt;
    use http::header::CONTENT_TYPE;
    use topcoat::view::{emit, live, view};

    use super::*;
    use crate::{
        BodyPanicError, LayoutFn, Method, PageFn, Router, RouterBuilder, Slot, error::redirect,
        to_bytes,
    };

    /// Dispatches a `GET` request for `path` through the router.
    async fn send(router: &Router, path: &str) -> Response {
        let request = http::Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        router.handle(request).await
    }

    /// Serves `render` as a page at `/p` and dispatches a `GET` to it.
    async fn send_page(render: crate::PageRenderFn) -> Response {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render))
            .build();
        send(&router, "/p").await
    }

    /// Reads the response body as its data frames, one string per frame.
    async fn data_frames(body: Body) -> Vec<String> {
        let mut frames = body.into_data_stream();
        let mut chunks = Vec::new();
        while let Some(frame) = frames.next().await {
            chunks.push(String::from_utf8(frame.unwrap().to_vec()).unwrap());
        }
        chunks
    }

    /// The envelope a swap for `region` arrives in.
    fn swap_envelope(region: u64, replacement: &str) -> String {
        format!(
            "<template data-topcoat-swap=\"{region}\">{replacement}</template>\
             <script>topcoat.swap({region})</script>"
        )
    }

    // Page and layout render functions, since `PageFn`/`LayoutFn` are backed
    // by plain `fn` pointers.

    /// A region that settles after a single emission.
    fn render_settled_region_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! { cx => <main>(live! { emit! { <p>"only"</p> } })</main> }.boxed()
    }

    /// A region that emits twice, so the response streams one swap.
    fn render_live_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    emit! { <p>"second"</p> }
                })
            </main>
        }
        .boxed()
    }

    /// A region that emits three times, so the response streams two swaps.
    fn render_thrice_emitting_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"one"</p> }?;
                    emit! { <p>"two"</p> }?;
                    emit! { <p>"three"</p> }
                })
            </main>
        }
        .boxed()
    }

    /// Two sibling regions that each emit twice.
    fn render_two_region_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                <section>
                    (live! {
                        emit! { <p>"a1"</p> }?;
                        emit! { <p>"a2"</p> }
                    })
                </section>
                <section>
                    (live! {
                        emit! { <p>"b1"</p> }?;
                        emit! { <p>"b2"</p> }
                    })
                </section>
            </main>
        }
        .boxed()
    }

    /// A live page that declares a status code and a header in its first
    /// content.
    fn render_live_metadata_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            (StatusCode::ACCEPTED)
            ((
                http::HeaderName::from_static("x-test"),
                http::HeaderValue::from_static("1"),
            ))
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    emit! { <p>"second"</p> }
                })
            </main>
        }
        .boxed()
    }

    /// A settled page that declares a status code and a header.
    fn render_settled_metadata_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            (StatusCode::CREATED)
            ((
                http::HeaderName::from_static("x-test"),
                http::HeaderValue::from_static("1"),
            ))
            <p>"made"</p>
        }
        .boxed()
    }

    /// A region that fails before it produces any content.
    fn render_failing_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! { cx => <main>(live! { Err(io::Error::other("boom").into()) })</main> }.boxed()
    }

    /// A region that fails after its first emission, mid-stream.
    ///
    /// The suspension point after the emission commits the response before
    /// the failure; a region that fails in the same poll as its emission
    /// fails the view before any content is sent.
    fn render_late_failing_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    tokio::task::yield_now().await;
                    Err(io::Error::other("late").into())
                })
            </main>
        }
        .boxed()
    }

    /// A region that panics after its first emission, mid-stream.
    fn render_late_panicking_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    tokio::task::yield_now().await;
                    panic!("late");
                })
            </main>
        }
        .boxed()
    }

    /// Wraps the child content in `R[ ... ]` so layout nesting is observable.
    fn wrap_layout<'a>(cx: &Cx, slot: Slot<'a>) -> BoxView<'a> {
        view! {
            cx =>
            "R["
            (slot)
            "]"
        }
        .boxed()
    }

    #[tokio::test]
    async fn a_page_with_a_settled_region_responds_with_plain_html() {
        let response = send_page(render_settled_region_page).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );

        // The region settled before the response, so no markers and no swap
        // applier reach the client.
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"<main><p>only</p></main>");
    }

    #[tokio::test]
    async fn a_live_page_streams_its_first_content_then_its_swaps() {
        let response = send_page(render_live_page).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );

        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 2);
        // The first frame is the initial document, the region marked off so
        // the swap can find it.
        assert_eq!(
            frames[0],
            "<main><!--topcoat::region::start(1)--><p>first</p>\
             <!--topcoat::region::end(1)--></main>"
        );
        // The swap arrives behind the applier, wrapped in a template the
        // applier splices between the markers.
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(1, "<p>second</p>"))
        );
    }

    #[tokio::test]
    async fn the_swap_applier_is_sent_once_ahead_of_the_first_swap() {
        let response = send_page(render_thrice_emitting_page).await;

        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 3);
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(1, "<p>two</p>"))
        );
        // Later swaps arrive bare: the applier is already installed.
        assert_eq!(frames[2], swap_envelope(1, "<p>three</p>"));
    }

    #[tokio::test]
    async fn sibling_regions_stream_their_own_swaps() {
        let response = send_page(render_two_region_page).await;

        let frames = data_frames(response.into_body()).await;
        // Both regions are marked off in the initial document.
        assert_eq!(
            frames[0],
            "<main>\
             <section><!--topcoat::region::start(1)--><p>a1</p>\
             <!--topcoat::region::end(1)--></section>\
             <section><!--topcoat::region::start(2)--><p>b1</p>\
             <!--topcoat::region::end(2)--></section>\
             </main>"
        );

        // Each region swaps once, and the two share a single applier.
        let swaps = frames[1..].concat();
        assert_eq!(swaps.matches("window.topcoat ??=").count(), 1);
        assert!(swaps.contains(&swap_envelope(1, "<p>a2</p>")), "{swaps}");
        assert!(swaps.contains(&swap_envelope(2, "<p>b2</p>")), "{swaps}");
    }

    #[tokio::test]
    async fn each_request_numbers_its_regions_from_the_start() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render_live_page))
            .build();

        for _ in 0..2 {
            let response = send(&router, "/p").await;
            let frames = data_frames(response.into_body()).await;
            assert!(
                frames[0].contains("<!--topcoat::region::start(1)-->"),
                "{}",
                frames[0]
            );
        }
    }

    #[tokio::test]
    async fn a_live_page_streams_below_its_layouts() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render_live_page))
            .layout(LayoutFn::new("/", wrap_layout))
            .build();

        let response = send(&router, "/p").await;
        let frames = data_frames(response.into_body()).await;
        assert_eq!(
            frames[0],
            "R[<main><!--topcoat::region::start(1)--><p>first</p>\
             <!--topcoat::region::end(1)--></main>]"
        );
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(1, "<p>second</p>"))
        );
    }

    #[tokio::test]
    async fn a_live_view_applies_its_declared_status_and_headers() {
        let response = send_page(render_live_metadata_page).await;
        // The metadata is collected from the first content, before the
        // response commits.
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(response.headers().get("x-test").unwrap(), "1");

        // The response still streams its swap.
        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 2);
        assert!(frames[1].ends_with(&swap_envelope(1, "<p>second</p>")));
    }

    #[tokio::test]
    async fn a_settled_view_applies_its_declared_status_and_headers() {
        let response = send_page(render_settled_metadata_page).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers().get("x-test").unwrap(), "1");

        // The declarations render no content of their own.
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"<p>made</p>");
    }

    #[tokio::test]
    async fn a_view_handle_response_carries_its_status_and_headers() {
        let cx = &Cx::default();
        let handle = view! {
            cx =>
            (StatusCode::CREATED)
            ((
                http::HeaderName::from_static("x-test"),
                http::HeaderValue::from_static("1"),
            ))
            <p>"made"</p>
        }
        .single()
        .await
        .unwrap();

        let response = handle.into_response(cx).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers().get("x-test").unwrap(), "1");
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"<p>made</p>");
    }

    #[tokio::test]
    async fn a_view_failing_before_its_first_content_is_a_server_error() {
        let response = send_page(render_failing_page).await;
        // The failure lands before the response commits, so the client gets
        // a proper error response.
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"internal server error");
    }

    /// A region that fails in the same poll as its first emission.
    fn render_immediately_failing_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    Err(io::Error::other("early").into())
                })
            </main>
        }
        .boxed()
    }

    #[tokio::test]
    async fn a_failure_in_the_same_poll_as_the_emission_is_a_server_error() {
        let response = send_page(render_immediately_failing_page).await;
        // The failure arrives while the region's liveness is still being
        // determined, before the response commits, so the emitted content is
        // discarded in favor of a proper error response.
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"internal server error");
    }

    #[tokio::test]
    async fn a_failure_after_the_first_content_ends_the_stream_with_an_error() {
        let response = send_page(render_late_failing_page).await;
        // The response committed with the first content, so the status can
        // no longer change; the failure surfaces from the body instead.
        assert_eq!(response.status(), StatusCode::OK);

        let mut frames = response.into_body().into_data_stream();
        let first = frames.next().await.unwrap().unwrap();
        assert!(first.starts_with(b"<main><!--topcoat::region::start(1)-->"));
        let error = frames.next().await.unwrap().unwrap_err();
        assert_eq!(error.to_string(), "late");
        // The failure ends the stream; the view is not polled again.
        assert!(frames.next().await.is_none());
    }

    #[tokio::test]
    async fn a_panic_after_the_first_content_ends_the_stream_with_an_error() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render_late_panicking_page))
            .page(PageFn::new(Method::GET, "/q", render_live_page))
            .build();

        let response = send(&router, "/p").await;
        assert_eq!(response.status(), StatusCode::OK);

        // The panic is caught at the body, so it ends the stream like a
        // failure does instead of unwinding into the connection.
        let mut frames = response.into_body().into_data_stream();
        let first = frames.next().await.unwrap().unwrap();
        assert!(first.starts_with(b"<main><!--topcoat::region::start(1)-->"));
        let error = frames.next().await.unwrap().unwrap_err();
        let error = error.downcast::<BodyPanicError>().unwrap();
        assert_eq!(error.message(), Some("late"));
        assert!(frames.next().await.is_none());

        // The router still serves other requests.
        let response = send(&router, "/q").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(data_frames(response.into_body()).await.len(), 2);
    }

    /// A region that redirects before it produces any content.
    fn render_redirecting_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! { cx => <main>(live! { Err(redirect("/target").into()) })</main> }.boxed()
    }

    /// A region that redirects after its first emission, mid-stream.
    fn render_late_redirecting_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    tokio::task::yield_now().await;
                    Err(redirect("/target").into())
                })
            </main>
        }
        .boxed()
    }

    #[tokio::test]
    async fn a_redirect_before_the_first_content_is_a_real_redirect() {
        let response = send_page(render_redirecting_page).await;
        // The redirect lands before the response commits, so the client gets
        // a proper redirect response.
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response.headers().get(http::header::LOCATION).unwrap(),
            "/target"
        );
    }

    #[tokio::test]
    async fn a_redirect_after_the_first_content_streams_a_navigation_script() {
        let response = send_page(render_late_redirecting_page).await;
        // The response committed with the first content, so the status can
        // no longer change; the redirect reaches the browser as a script.
        assert_eq!(response.status(), StatusCode::OK);

        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 2);
        assert!(frames[0].starts_with("<main><!--topcoat::region::start(1)-->"));
        assert_eq!(
            frames[1],
            "<script>window.location.replace(\"/target\")</script>"
        );
    }

    #[test]
    fn the_navigation_script_escapes_the_redirect_target() {
        let script = redirect_script(&redirect("/a\"b\\c<d"));
        assert_eq!(
            script,
            "<script>window.location.replace(\"/a\\\"b\\\\c\\x3Cd\")</script>"
        );
    }
}
