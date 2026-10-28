use topcoat_core::context::Cx;
use topcoat_router::request::try_headers;

use crate::header;

/// Returns whether the request carries `Datastar-Request: true`.
#[inline]
#[must_use]
pub fn datastar_request(cx: &Cx) -> bool {
    try_headers(cx)
        .and_then(|headers| headers.get(&header::DATASTAR_REQUEST))
        .and_then(|value| value.to_str().ok())
        == Some("true")
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::Cx;

    use super::*;

    fn cx_with(headers: &[(&http::HeaderName, &str)]) -> Cx {
        let mut builder = Request::builder();
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }

        let (parts, ()) = builder.body(()).unwrap().into_parts();
        Cx::new().with(parts)
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
        assert!(!datastar_request(&Cx::new()));
    }
}
