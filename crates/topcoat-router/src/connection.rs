use std::net::SocketAddr;

/// The IP address and port of the direct connection, stored in the request
/// [`extensions`](crate::request::extensions).
///
/// Topcoat adds this to every request received over TCP. Read it with
/// [`remote_addr`](crate::request::remote_addr). Behind a reverse proxy,
/// this is the proxy's address.
///
/// Unix socket connections have no IP address, so Topcoat does not add this
/// value for them. If you serve the router through a tower service or pass
/// requests to it yourself, insert this value into each request's extensions
/// to make the connection's address available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteAddr(pub SocketAddr);

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::{Cx, CxTestBuilder};

    use super::*;
    use crate::request::remote_addr;

    /// Builds a `Cx` for a request carrying `remote`, if any.
    fn cx_with(remote: Option<SocketAddr>) -> Cx {
        let mut builder = Request::builder().uri("/x");
        if let Some(addr) = remote {
            builder = builder.extension(RemoteAddr(addr));
        }
        let (parts, ()) = builder.body(()).expect("request should build").into_parts();
        CxTestBuilder::new().request_context(parts).build()
    }

    #[test]
    fn reads_the_peer_address_off_the_request() {
        let addr: SocketAddr = "203.0.113.9:4242".parse().unwrap();
        assert_eq!(remote_addr(&cx_with(Some(addr))), Some(addr));
    }

    #[test]
    fn a_request_without_a_peer_address_has_none() {
        assert_eq!(remote_addr(&cx_with(None)), None);
    }

    #[test]
    fn a_context_without_a_request_has_none() {
        assert_eq!(remote_addr(&CxTestBuilder::new().build()), None);
    }
}
