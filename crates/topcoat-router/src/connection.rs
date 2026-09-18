use std::net::SocketAddr;

use topcoat_core::context::Cx;

use crate::request::extensions;

/// The socket address of the peer a request's connection was accepted from,
/// carried in the request [`extensions`].
///
/// The server inserts it on every request of a connection accepted over TCP.
/// A request that arrives another way carries none unless whoever hands it to
/// the router inserts one: a connection over a Unix socket has no socket
/// address, and a tower service embedding the router passes the request
/// through as it is. Read it with [`remote_addr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteAddr(pub SocketAddr);

/// Returns the socket address of the peer the current request's connection
/// was accepted from, or `None` when it is unknown.
///
/// Behind a reverse proxy this is the proxy's address, not the client's; use
/// [`client_ip`](crate::client_ip) for the client's. The address is unknown
/// for a connection over a Unix socket, or a request that reached the router
/// without a [`RemoteAddr`] in its extensions.
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
