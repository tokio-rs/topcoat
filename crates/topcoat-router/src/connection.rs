use std::net::SocketAddr;

use topcoat_core::context::Cx;

use crate::request::extensions;

/// The IP address and port of the direct connection, stored in the request
/// [`extensions`].
///
/// Topcoat adds this to every request received over TCP. Read it with
/// [`remote_addr`]. Behind a reverse proxy, this is the proxy's address.
///
/// Unix socket connections have no IP address, so Topcoat does not add this
/// value for them. If you serve the router through a tower service or pass
/// requests to it yourself, insert this value into each request's extensions
/// to make the connection's address available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteAddr(pub SocketAddr);

/// Returns the IP address and port of the direct connection for this request,
/// or `None` when they are unknown.
///
/// Behind a reverse proxy, this returns the proxy's address. Use
/// [`client_ip`](crate::client_ip) to read the client's IP address instead.
/// Returns `None` if the request has no [`RemoteAddr`] in its extensions,
/// as is normally the case for Unix socket connections.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::remote_addr};
///
/// fn peer_port(cx: &Cx) -> Option<u16> {
///     remote_addr(cx).map(|addr| addr.port())
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn remote_addr(cx: &Cx) -> Option<SocketAddr> {
    extensions(cx).get::<RemoteAddr>().map(|remote| remote.0)
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

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
}
