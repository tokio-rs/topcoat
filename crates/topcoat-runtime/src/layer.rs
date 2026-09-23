use serde::Deserialize;
use topcoat_core::context::Cx;
use topcoat_router::{
    Body, HeaderName, HeaderValue, Layer, LayerFuture, Method, Next, Path,
    content::Json,
    error::rewrite,
    header,
    request::{FromRequest, headers, method, uri},
};

use crate::SignalValues;

/// The header used to request a page rerun.
///
/// A `POST` with `X-Topcoat-Runtime: true` is handled by [`RuntimeLayer`].
pub static RUNTIME_HEADER: HeaderName = HeaderName::from_static("x-topcoat-runtime");

/// The header value that identifies a page rerun.
static RERUN: HeaderValue = HeaderValue::from_static("true");

/// The document's current signal values, sent as the JSON body of a page rerun.
#[derive(Debug, Deserialize)]
struct PageRerunRequest {
    #[serde(default)]
    signals: SignalValues,
}

/// A [`Layer`] that reruns pages with signal values from the browser.
///
/// The browser sends a `POST` to the page's URL with
/// `X-Topcoat-Runtime: true` and a JSON body containing the document's
/// signal values. This layer rewrites the request to a `GET` at the same
/// path and query. The page runs through its layouts and guards, and its
/// signals resume from the supplied values.
///
/// The rewritten request has an empty body. Its [`RUNTIME_HEADER`],
/// `Content-Type`, and `Content-Length` headers are removed. Read the
/// client's original method with
/// [`original_method`](topcoat_router::request::original_method).
/// Requests with another method or without `X-Topcoat-Runtime: true`
/// pass through unchanged, including ordinary form submissions.
///
/// [`RouterBuilderRuntimeExt::runtime`](crate::RouterBuilderRuntimeExt::runtime)
/// registers this layer. Call it after registering your application's
/// pathless layers so those layers receive the rewritten `GET`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeLayer;

impl Layer for RuntimeLayer {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        let marked = headers(cx).get(&RUNTIME_HEADER) == Some(&RERUN);
        if *method(cx) != Method::POST || !marked {
            return next.run(cx, body);
        }
        Box::pin(async move {
            let Json(request) = Json::<PageRerunRequest>::from_request(cx, body).await?;

            // Remove the rerun marker and body headers before dispatching
            // the GET with an empty body.
            let mut headers = headers(cx).clone();
            headers.remove(&RUNTIME_HEADER);
            headers.remove(header::CONTENT_TYPE);
            headers.remove(header::CONTENT_LENGTH);

            let uri = uri(cx);
            let target = uri
                .path_and_query()
                .map_or(uri.path(), |path_and_query| path_and_query.as_str());
            Err(rewrite(target, Body::empty())
                .method(Method::GET)
                .headers(headers)
                .with(request.signals)
                .into())
        })
    }
}
