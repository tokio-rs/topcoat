//! The HTTP transport of the runtime protocol: a page rerun posted to the
//! page's own URL.

use serde::Deserialize;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, HeaderName, HeaderValue, Method,
    content::Json,
    error::rewrite,
    header,
    request::{FromRequest, headers, method, uri},
    response::Response,
};

use crate::SignalValues;

/// The header used to request a page rerun.
///
/// A `POST` with `X-Topcoat-Runtime: true` is handled by
/// [`RuntimeLayer`](crate::RuntimeLayer).
pub static RUNTIME_HEADER: HeaderName = HeaderName::from_static("x-topcoat-runtime");

/// The header value that identifies a page rerun.
static RERUN: HeaderValue = HeaderValue::from_static("true");

/// The document's current signal values, sent as the JSON body of a page rerun.
#[derive(Debug, Deserialize)]
struct PageRerunRequest {
    #[serde(default)]
    signals: SignalValues,
}

/// Whether the request is a page rerun: a `POST` carrying the marker.
pub(super) fn requested(cx: &Cx) -> bool {
    *method(cx) == Method::POST && headers(cx).get(&RUNTIME_HEADER) == Some(&RERUN)
}

/// Reads the rerun's signal values and rewrites the request to a `GET` at
/// the same path and query carrying them.
///
/// The rewrite is returned as the error it travels as, so this never
/// produces a response of its own.
pub(super) async fn dispatch(cx: &Cx, body: Body) -> Result<Response> {
    let Json(request) = Json::<PageRerunRequest>::from_request(cx, body).await?;

    // Remove the rerun marker and body headers before dispatching the GET
    // with an empty body.
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
}
