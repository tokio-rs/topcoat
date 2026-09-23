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

/// The header marking a request as sent by the browser runtime.
pub static RUNTIME_HEADER: HeaderName = HeaderName::from_static("x-topcoat-runtime");

/// The value of [`RUNTIME_HEADER`] on a page re-run.
static RERUN: HeaderValue = HeaderValue::from_static("true");

/// The body of a request re-running a page: the current values of the
/// signals in the document.
#[derive(Debug, Deserialize)]
struct PageRerunRequest {
    #[serde(default)]
    signals: SignalValues,
}

/// The [`Layer`] serving the browser runtime's requests at a page's own URL.
///
/// A page re-run is a `POST` to the page URL that carries
/// [`RUNTIME_HEADER`] and the document's signal values as JSON. The layer
/// rewrites it as a `GET` for the same path and query, with the signal
/// values in the request context and the headers describing the consumed
/// body removed. The page then runs through its layouts and guards like any
/// other request, and its signals resume from the supplied values. The
/// original method stays readable through
/// [`original_method`](topcoat_router::request::original_method).
///
/// A request without the header passes through untouched, so a `POST`
/// from a form still reaches its handler.
///
/// [`RouterBuilderRuntimeExt::runtime`](crate::RouterBuilderRuntimeExt::runtime)
/// registers this layer. It wraps every layer registered before it, so
/// those layers only ever see the rewritten `GET`.
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

            // The next dispatch gets an empty body, so nothing may describe
            // the envelope this one consumed.
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
