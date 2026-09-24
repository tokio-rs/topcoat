//! Renders a page again when the browser sends a runtime POST to its URL.

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

/// The header that identifies an HTTP page rerun.
///
/// Set `X-Topcoat-Runtime: true` on a `POST` request for
/// [`RuntimeLayer`](crate::RuntimeLayer) to handle it as a page rerun.
pub static RUNTIME_HEADER: HeaderName = HeaderName::from_static("x-topcoat-runtime");

/// The value required in the runtime header.
static RERUN: HeaderValue = HeaderValue::from_static("true");

/// The JSON request body containing the browser's signal values.
#[derive(Debug, Deserialize)]
struct PageRerunRequest {
    #[serde(default)]
    signals: SignalValues,
}

/// Checks for a `POST` with the runtime header set to `true`.
pub(super) fn requested(cx: &Cx) -> bool {
    *method(cx) == Method::POST && headers(cx).get(&RUNTIME_HEADER) == Some(&RERUN)
}

/// Reads the signal values and rewrites the request as a `GET` at the same
/// path and query, with those values in its context.
///
/// Returns the rewrite as an error for the router to handle.
pub(super) async fn dispatch(cx: &Cx, body: Body) -> Result<Response> {
    let Json(request) = Json::<PageRerunRequest>::from_request(cx, body).await?;

    // The rewritten GET has no body and is no longer a rerun request.
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
