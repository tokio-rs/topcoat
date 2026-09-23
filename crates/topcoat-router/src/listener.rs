use std::{future::Future, io, net::SocketAddr};

#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};

/// A bound socket that the serve functions accept connections from.
///
/// Implement it to serve a router over another connection-based transport.
/// Topcoat implements it for [`TcpListener`] and, on Unix, for
/// [`UnixListener`].
pub trait Listener: Send + 'static {
    /// The stream of an accepted connection.
    type Io: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    /// Waits for the next connection and returns its stream and the remote IP
    /// address and port.
    ///
    /// Topcoat stores the address as [`RemoteAddr`](crate::RemoteAddr) in
    /// every request received over the connection. Return `None` as the
    /// address for connections without an IP address, such as Unix sockets.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if accepting the connection fails.
    fn accept(&mut self)
    -> impl Future<Output = io::Result<(Self::Io, Option<SocketAddr>)>> + Send;

    /// Returns the local TCP address the listener is bound to, if it has one.
    ///
    /// The serve functions tell the `topcoat dev` server this address so it
    /// can forward requests to the application. The default implementation
    /// returns `None`, which is right for listeners without a TCP address,
    /// like Unix sockets.
    fn tcp_addr(&self) -> Option<SocketAddr> {
        None
    }
}

impl Listener for TcpListener {
    type Io = tokio::net::TcpStream;

    async fn accept(&mut self) -> io::Result<(Self::Io, Option<SocketAddr>)> {
        let (stream, addr) = TcpListener::accept(self).await?;
        Ok((stream, Some(addr)))
    }

    fn tcp_addr(&self) -> Option<SocketAddr> {
        TcpListener::local_addr(self).ok()
    }
}

/// Serves over a Unix socket, usually behind a reverse proxy that forwards
/// HTTP requests to the socket path.
///
/// Unix socket connections have no IP address, so requests have no
/// [`RemoteAddr`](crate::RemoteAddr). If a reverse proxy connects through
/// this socket, use [`TrustedProxies::nearest`](crate::TrustedProxies::nearest)
/// to trust it and read the client's IP address from its headers.
///
/// Binding fails with `AddrInUse` if the socket file already exists, and
/// dropping the listener does not remove the file. So remove any file left
/// over from a previous run before binding:
///
/// ```no_run
/// # #[cfg(unix)]
/// # fn bind() -> std::io::Result<tokio::net::UnixListener> {
/// let path = "/run/my-app.sock";
/// let _ = std::fs::remove_file(path);
/// let listener = tokio::net::UnixListener::bind(path)?;
/// # Ok(listener)
/// # }
/// ```
#[cfg(unix)]
impl Listener for UnixListener {
    type Io = tokio::net::UnixStream;

    async fn accept(&mut self) -> io::Result<(Self::Io, Option<SocketAddr>)> {
        let (stream, _addr) = UnixListener::accept(self).await?;
        Ok((stream, None))
    }
}
