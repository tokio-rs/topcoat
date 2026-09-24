use std::{
    fmt::Write,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::future::poll_fn;
use http::{HeaderMap, HeaderValue, StatusCode};
use http_body::Frame;
use pin_project_lite::pin_project;
use serde::Serialize;
use topcoat_core::{
    context::{Cx, try_request_context},
    error::Result,
};
use topcoat_view::{
    BoxView, Formatter, RegionId, View, ViewExt, ViewHandle, ViewSwap, internal::MoveView,
};

use crate::{
    Body, BoxError,
    error::redirect_location,
    response::{AsyncIntoResponse, IntoResponse, Response},
};

/// The media type of a framed view response.
const FRAMES_MEDIA_TYPE: &str = "application/x-ndjson";

/// The format used to send a view response to the browser.
///
/// Add this value to the request context to choose a format. Without one,
/// a request whose `Accept` header lists `application/x-ndjson` gets
/// [`Frames`](Self::Frames) and any other request gets
/// [`Html`](Self::Html). Both formats use the view's status code and
/// headers. A redirect or error before the view produces any content
/// becomes a normal HTTP redirect or error response.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewResponseDelivery {
    /// Sends HTML for the browser to display.
    ///
    /// The response starts with the document. Later live region updates
    /// include a template with the new content and a script to apply it.
    /// A redirect after the document starts uses a script to navigate.
    #[default]
    Html,
    /// Sends JSON frames for the browser runtime to apply, one per line.
    ///
    /// The content type is `application/x-ndjson`. The `t` field identifies
    /// the frame type. A `snapshot` contains the initial `html`. A `swap`
    /// contains a `region` id and the `html` to put in that region. A
    /// `redirect` contains the `location` to navigate to after content has
    /// already been sent.
    Frames,
}

/// Returns the view response format for this request.
///
/// A format set in the request context wins. Otherwise the request's
/// `Accept` header decides: [`ViewResponseDelivery::Frames`] when it lists
/// `application/x-ndjson`, and [`ViewResponseDelivery::Html`] when it does
/// not or there is no request.
#[must_use]
pub fn view_response_delivery(cx: &Cx) -> ViewResponseDelivery {
    if let Some(delivery) = try_request_context::<ViewResponseDelivery>(cx) {
        return *delivery;
    }
    let accepts_frames = try_request_context::<http::request::Parts>(cx)
        .and_then(|parts| parts.headers.get(http::header::ACCEPT))
        .and_then(|accept| accept.to_str().ok())
        .is_some_and(|accept| {
            accept.split(',').any(|entry| {
                let media_type = entry.split(';').next().unwrap_or(entry).trim();
                media_type.eq_ignore_ascii_case(FRAMES_MEDIA_TYPE)
            })
        });
    if accepts_frames {
        ViewResponseDelivery::Frames
    } else {
        ViewResponseDelivery::Html
    }
}

impl ViewResponseDelivery {
    /// Returns the content type for this format.
    fn content_type(self) -> HeaderValue {
        HeaderValue::from_static(match self {
            Self::Html => "text/html; charset=utf-8",
            Self::Frames => FRAMES_MEDIA_TYPE,
        })
    }

    /// Formats the initial HTML as a response body chunk.
    fn first(self, html: String) -> String {
        match self {
            Self::Html => html,
            Self::Frames => ViewFrame::Snapshot { html: &html }.to_line(),
        }
    }

    /// Formats a live region update as a response body chunk.
    /// For HTML, `script` contains the update script if it is still needed.
    fn swap(self, cx: &Cx, swap: ViewSwap, script: Option<&'static str>) -> String {
        let region = swap.region;
        match self {
            Self::Html => {
                // Allow room for the content, its wrapper, both region ids,
                // and the optional script.
                let mut envelope = String::with_capacity(
                    script.map_or(0, str::len) + swap.replacement.size_hint() + 96,
                );
                let mut f = Formatter::new(&mut envelope);
                if let Some(script) = script {
                    f.write_str(script);
                }
                write!(f, "<template data-topcoat-swap=\"{region}\">").unwrap();
                swap.replacement.render_into(cx, &mut f);
                write!(f, "</template><script>topcoat.swap(\"{region}\")</script>").unwrap();
                envelope
            }
            Self::Frames => {
                let mut html = String::with_capacity(swap.replacement.size_hint());
                swap.replacement
                    .render_into(cx, &mut Formatter::new(&mut html));
                ViewFrame::Swap {
                    region,
                    html: &html,
                }
                .to_line()
            }
        }
    }

    /// Formats a redirect that occurs after the initial content was sent.
    fn redirect(self, location: &HeaderValue) -> String {
        match self {
            Self::Html => redirect_script(location),
            Self::Frames => {
                // Percent-encoding makes the location valid ASCII.
                let location = location.to_str().expect("redirect location is ASCII");
                ViewFrame::Redirect { location }.to_line()
            }
        }
    }
}

/// A JSON frame sent with [`ViewResponseDelivery::Frames`].
#[derive(Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ViewFrame<'a> {
    Snapshot {
        html: &'a str,
    },
    Swap {
        #[serde(serialize_with = "serialize_region")]
        region: RegionId,
        html: &'a str,
    },
    Redirect {
        location: &'a str,
    },
}

/// Writes a region id in the same format as its HTML markers.
fn serialize_region<S: serde::Serializer>(
    region: &RegionId,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.collect_str(region)
}

impl ViewFrame<'_> {
    /// Encodes a frame as JSON followed by a newline.
    fn to_line(&self) -> String {
        let mut line = serde_json::to_string(self).expect("a view frame serializes");
        line.push('\n');
        line
    }
}

impl IntoResponse for ViewHandle {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let delivery = view_response_delivery(cx);
        let rendered = self.render_response(cx);
        view_response(
            cx,
            delivery.first(rendered.html),
            rendered.status_code,
            rendered.headers,
            delivery,
        )
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
    let delivery = view_response_delivery(cx);
    let mut pinned_view = Pin::new(&mut view);
    let first = poll_fn(|cx| pinned_view.as_mut().poll_first(cx)).await?;
    let rendered = first.content.render_response(cx);
    let first_content = delivery.first(rendered.html);
    if first.live {
        let body = ViewBody {
            cx: cx.clone(),
            first: Some(first_content),
            script: (delivery == ViewResponseDelivery::Html).then_some(SWAP_SCRIPT),
            delivery,
            done: false,
            view,
        };
        view_response(
            cx,
            Body::new(body),
            rendered.status_code,
            rendered.headers,
            delivery,
        )
    } else {
        view_response(
            cx,
            Body::new(first_content),
            rendered.status_code,
            rendered.headers,
            delivery,
        )
    }
}

/// Builds a response with the chosen format's content type and the view's
/// status code and headers.
fn view_response(
    cx: &Cx,
    body: impl Into<Body>,
    status_code: Option<StatusCode>,
    headers: HeaderMap,
    delivery: ViewResponseDelivery,
) -> Result<Response> {
    let mut response = (
        [(http::header::CONTENT_TYPE, delivery.content_type())],
        body.into(),
    )
        .into_response(cx)?;
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
            if (comment.data === `::topcoat::region::start(${id})`) open = comment;
            else if (comment.data === `::topcoat::region::end(${id})`) close = comment;
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
/// redirect's `location`. `replace` keeps the partially streamed page out of
/// the session history, so going back skips it.
fn redirect_script(location: &HeaderValue) -> String {
    // The location is percent-encoded down to ASCII, so it converts back.
    let uri = location.to_str().expect("redirect location is ASCII");
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
        // HTML responses send this script once, before the first update.
        script: Option<&'static str>,
        delivery: ViewResponseDelivery,
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
                let chunk = this.delivery.swap(this.cx, swap, this.script.take());
                Poll::Ready(Some(Ok(Frame::data(chunk.into()))))
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
                match redirect_location(error) {
                    Ok(location) => {
                        let chunk = this.delivery.redirect(&location);
                        Poll::Ready(Some(Ok(Frame::data(chunk.into()))))
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
        BodyPanicError, LayoutFn, Method, PageFn, Router, RouterBuilder, Slot,
        error::{redirect, see_other},
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

    /// Requests `/p` using JSON frames, with `render` as its page handler.
    async fn send_page_framed(render: crate::PageRenderFn) -> Response {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render))
            .build();
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("/p")
            .body(Body::empty())
            .unwrap();
        router
            .handle_with(request, (ViewResponseDelivery::Frames,))
            .await
    }

    /// Parses the response frames and checks that each body chunk contains
    /// one JSON object followed by a newline.
    async fn json_frames(body: Body) -> Vec<serde_json::Value> {
        data_frames(body)
            .await
            .into_iter()
            .map(|frame| {
                let line = frame
                    .strip_suffix('\n')
                    .expect("a frame ends with a newline");
                assert!(!line.contains('\n'), "{frame}");
                serde_json::from_str(line).unwrap()
            })
            .collect()
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
    fn swap_envelope(region: &str, replacement: &str) -> String {
        format!(
            "<template data-topcoat-swap=\"{region}\">{replacement}</template>\
             <script>topcoat.swap(\"{region}\")</script>"
        )
    }

    fn region_ids(html: &str) -> Vec<&str> {
        html.split("<!--::topcoat::region::start(")
            .skip(1)
            .map(|part| {
                let (id, _) = part.split_once(")-->").unwrap();
                assert_eq!(id.len(), 32);
                assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()));
                id
            })
            .collect()
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
        let region = region_ids(&frames[0])[0];
        assert_eq!(
            frames[0],
            format!(
                "<main><!--::topcoat::region::start({region})--><p>first</p>\
             <!--::topcoat::region::end({region})--></main>"
            )
        );
        // The swap arrives behind the applier, wrapped in a template the
        // applier splices between the markers.
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(region, "<p>second</p>"))
        );
    }

    #[tokio::test]
    async fn the_swap_applier_is_sent_once_ahead_of_the_first_swap() {
        let response = send_page(render_thrice_emitting_page).await;

        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 3);
        let region = region_ids(&frames[0])[0];
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(region, "<p>two</p>"))
        );
        // Later swaps arrive bare: the applier is already installed.
        assert_eq!(frames[2], swap_envelope(region, "<p>three</p>"));
    }

    #[tokio::test]
    async fn sibling_regions_stream_their_own_swaps() {
        let response = send_page(render_two_region_page).await;

        let frames = data_frames(response.into_body()).await;
        // Both regions are marked off in the initial document.
        let ids = region_ids(&frames[0]);
        assert_eq!(ids.len(), 2);
        let (a, b) = (ids[0], ids[1]);
        assert_ne!(a, b);
        assert_eq!(
            frames[0],
            format!(
                "<main>\
             <section><!--::topcoat::region::start({a})--><p>a1</p>\
             <!--::topcoat::region::end({a})--></section>\
             <section><!--::topcoat::region::start({b})--><p>b1</p>\
             <!--::topcoat::region::end({b})--></section>\
             </main>"
            )
        );

        // Each region swaps once, and the two share a single applier.
        let swaps = frames[1..].concat();
        assert_eq!(swaps.matches("window.topcoat ??=").count(), 1);
        assert!(swaps.contains(&swap_envelope(a, "<p>a2</p>")), "{swaps}");
        assert!(swaps.contains(&swap_envelope(b, "<p>b2</p>")), "{swaps}");
    }

    #[tokio::test]
    async fn region_ids_are_stable_across_requests() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render_live_page))
            .build();

        let mut previous = None;
        for _ in 0..2 {
            let response = send(&router, "/p").await;
            let frames = data_frames(response.into_body()).await;
            let region = region_ids(&frames[0])[0];
            if let Some(previous) = &previous {
                assert_eq!(region, previous);
            }
            previous = Some(region.to_owned());
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
        let region = region_ids(&frames[0])[0];
        assert_eq!(
            frames[0],
            format!(
                "R[<main><!--::topcoat::region::start({region})--><p>first</p>\
             <!--::topcoat::region::end({region})--></main>]"
            )
        );
        assert_eq!(
            frames[1],
            format!("{SWAP_SCRIPT}{}", swap_envelope(region, "<p>second</p>"))
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
        let region = region_ids(&frames[0])[0];
        assert!(frames[1].ends_with(&swap_envelope(region, "<p>second</p>")));
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
        assert!(first.starts_with(b"<main><!--::topcoat::region::start("));
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
        assert!(first.starts_with(b"<main><!--::topcoat::region::start("));
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

    /// A page that answers with a "see other" before it produces any content.
    fn render_see_other_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! { cx => <main>(live! { Err(see_other("/target").into()) })</main> }.boxed()
    }

    /// A page that answers with a "see other" after its first emission.
    fn render_late_see_other_page(cx: &Cx, _body: Body) -> BoxView<'_> {
        view! {
            cx =>
            <main>
                (live! {
                    emit! { <p>"first"</p> }?;
                    tokio::task::yield_now().await;
                    Err(see_other("/target").into())
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
        assert!(frames[0].starts_with("<main><!--::topcoat::region::start("));
        assert_eq!(
            frames[1],
            "<script>window.location.replace(\"/target\")</script>"
        );
    }

    #[tokio::test]
    async fn a_see_other_before_the_first_content_is_a_real_redirect() {
        let response = send_page(render_see_other_page).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(http::header::LOCATION).unwrap(),
            "/target"
        );
    }

    #[tokio::test]
    async fn a_see_other_after_the_first_content_streams_a_navigation_script() {
        let response = send_page(render_late_see_other_page).await;
        assert_eq!(response.status(), StatusCode::OK);

        let frames = data_frames(response.into_body()).await;
        assert_eq!(frames.len(), 2);
        assert_eq!(
            frames[1],
            "<script>window.location.replace(\"/target\")</script>"
        );
    }

    #[test]
    fn the_navigation_script_escapes_the_redirect_target() {
        let script = redirect_script(redirect("/a\"b\\c<d").location());
        assert_eq!(
            script,
            "<script>window.location.replace(\"/a\\\"b\\\\c\\x3Cd\")</script>"
        );
    }

    #[test]
    fn the_navigation_script_keeps_a_non_ascii_redirect_target() {
        let script = redirect_script(redirect("/caf\u{e9}").location());
        assert_eq!(
            script,
            "<script>window.location.replace(\"/caf%C3%A9\")</script>"
        );
    }

    #[test]
    fn a_request_without_a_delivery_gets_html() {
        assert_eq!(
            view_response_delivery(&Cx::default()),
            ViewResponseDelivery::Html
        );
        assert_eq!(
            view_response_delivery(&Cx::default().with(ViewResponseDelivery::Frames)),
            ViewResponseDelivery::Frames
        );
    }

    /// Requests `/p` with the given `Accept` header, with `render` as its
    /// page handler.
    async fn send_page_accepting(render: crate::PageRenderFn, accept: &str) -> Response {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, "/p", render))
            .build();
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("/p")
            .header(http::header::ACCEPT, accept)
            .body(Body::empty())
            .unwrap();
        router.handle(request).await
    }

    #[tokio::test]
    async fn a_request_accepting_frames_gets_them() {
        let response =
            send_page_accepting(render_thrice_emitting_page, "text/html, application/x-ndjson;q=0.9")
                .await;
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );
        let frames = json_frames(response.into_body()).await;
        assert_eq!(frames[0]["t"], "snapshot");
        assert_eq!(frames.len(), 3, "{frames:?}");
    }

    #[tokio::test]
    async fn a_request_accepting_only_html_gets_html() {
        let response = send_page_accepting(render_settled_region_page, "text/html").await;
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn a_delivery_in_the_context_wins_over_the_accept_header() {
        let router = RouterBuilder::new()
            .page(PageFn::new(
                Method::GET,
                "/p",
                render_settled_region_page,
            ))
            .build();
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("/p")
            .header(http::header::ACCEPT, "application/x-ndjson")
            .body(Body::empty())
            .unwrap();
        let response = router
            .handle_with(request, (ViewResponseDelivery::Html,))
            .await;
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn a_framed_settled_page_is_one_snapshot_frame() {
        let response = send_page_framed(render_settled_region_page).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );

        let frames = json_frames(response.into_body()).await;
        assert_eq!(
            frames,
            [serde_json::json!({ "t": "snapshot", "html": "<main><p>only</p></main>" })]
        );
    }

    #[tokio::test]
    async fn a_framed_live_page_sends_its_snapshot_then_one_frame_per_swap() {
        let response = send_page_framed(render_thrice_emitting_page).await;
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );

        let raw = data_frames(response.into_body()).await;
        // JSON frames need no scripts because the runtime applies them.
        assert!(
            raw.iter().all(|frame| !frame.contains("<script")),
            "{raw:?}"
        );

        let frames: Vec<serde_json::Value> = raw
            .iter()
            .map(|frame| serde_json::from_str(frame.trim_end()).unwrap())
            .collect();
        assert_eq!(frames.len(), 3);
        // Later updates must refer to a region in the initial HTML.
        let html = frames[0]["html"].as_str().unwrap();
        let region = region_ids(html)[0];
        assert_eq!(frames[0]["t"], "snapshot");
        assert!(html.contains("<p>one</p>"), "{html}");
        assert_eq!(
            frames[1],
            serde_json::json!({ "t": "swap", "region": region, "html": "<p>two</p>" })
        );
        assert_eq!(
            frames[2],
            serde_json::json!({ "t": "swap", "region": region, "html": "<p>three</p>" })
        );
    }

    #[tokio::test]
    async fn framed_sibling_regions_name_their_own_region() {
        let response = send_page_framed(render_two_region_page).await;

        let frames = json_frames(response.into_body()).await;
        let html = frames[0]["html"].as_str().unwrap();
        let regions = region_ids(html);
        assert_eq!(regions.len(), 2);
        // Each update must target one of the page's two regions.
        for frame in &frames[1..] {
            assert_eq!(frame["t"], "swap");
            let region = frame["region"].as_str().unwrap();
            assert!(regions.contains(&region), "{frame}");
        }
    }

    #[tokio::test]
    async fn a_framed_redirect_after_the_first_content_is_a_redirect_frame() {
        let response = send_page_framed(render_late_redirecting_page).await;
        assert_eq!(response.status(), StatusCode::OK);

        let frames = json_frames(response.into_body()).await;
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["t"], "snapshot");
        assert_eq!(
            frames[1],
            serde_json::json!({ "t": "redirect", "location": "/target" })
        );
    }

    #[tokio::test]
    async fn a_framed_redirect_before_the_first_content_is_a_real_redirect() {
        let response = send_page_framed(render_redirecting_page).await;
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response.headers().get(http::header::LOCATION).unwrap(),
            "/target"
        );
    }

    #[tokio::test]
    async fn a_framed_view_handle_response_is_a_snapshot_frame_with_its_status() {
        let cx = &Cx::default().with(ViewResponseDelivery::Frames);
        let handle = view! {
            cx =>
            (StatusCode::CREATED)
            <p>"made"</p>
        }
        .single()
        .await
        .unwrap();

        let response = handle.into_response(cx).unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/x-ndjson"
        );
        let frames = json_frames(response.into_body()).await;
        assert_eq!(
            frames,
            [serde_json::json!({ "t": "snapshot", "html": "<p>made</p>" })]
        );
    }
}
