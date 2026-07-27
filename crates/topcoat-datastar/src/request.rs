use topcoat_core::context::Cx;
use topcoat_router::headers;

use crate::header;

/// Returns `true` when the current request was issued by a Datastar action,
/// i.e. it carries a `Datastar-Request: true` header.
///
/// # Panics
///
/// Panics if called outside a router request (no request
/// [`Parts`](http::request::Parts) in context).
#[inline]
#[must_use]
pub fn datastar_request(cx: &Cx) -> bool {
    headers(cx)
        .get(&header::DATASTAR_REQUEST)
        .and_then(|value| value.to_str().ok())
        == Some("true")
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    fn cx_with(headers: &[(&http::HeaderName, &str)]) -> Cx {
        let mut builder = Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }

        let (parts, ()) = builder.body(()).unwrap().into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    #[test]
    fn datastar_request_requires_the_header_to_be_true() {
        assert!(datastar_request(&cx_with(&[(
            &header::DATASTAR_REQUEST,
            "true"
        )])));
        assert!(!datastar_request(&cx_with(&[(
            &header::DATASTAR_REQUEST,
            "false"
        )])));
        assert!(!datastar_request(&cx_with(&[])));
    }
}
