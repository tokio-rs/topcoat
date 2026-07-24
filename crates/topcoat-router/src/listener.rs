use std::future::Future;
use std::io;
use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;

/// A bound listener the serve functions can accept connections from.
///
/// The serve functions are generic over this trait, so a router can be served
/// over any connection-oriented transport. Implementations are provided for
/// [`TcpListener`] and, on Unix, [`UnixListener`].
pub trait Listener: Send + 'static {
    /// The I/O stream of an accepted connection.
    type Io: AsyncRead + AsyncWrite + Unpin + Send + 'static;

    /// The peer address of an accepted connection.
    type Addr: Send;

    /// Accepts the next inbound connection, yielding its I/O stream and the
    /// peer's address.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if accepting the connection fails.
    fn accept(&mut self) -> impl Future<Output = io::Result<(Self::Io, Self::Addr)>> + Send;

    /// The local TCP address the listener is bound to, when it has one.
    ///
    /// The serve functions report this address to the `topcoat dev` server so
    /// it can proxy to the application. Listeners without a TCP address, like
    /// Unix domain sockets, return `None`, the default.
    fn tcp_addr(&self) -> Option<SocketAddr> {
        None
    }
}

impl Listener for TcpListener {
    type Io = tokio::net::TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> io::Result<(Self::Io, Self::Addr)> {
        TcpListener::accept(self).await
    }

    fn tcp_addr(&self) -> Option<SocketAddr> {
        TcpListener::local_addr(self).ok()
    }
}

/// Serves over a Unix domain socket, typically behind a reverse proxy that
/// forwards HTTP to the socket path.
///
/// Binding fails with `AddrInUse` if the socket file already exists, and
/// dropping the listener does not remove it, so remove any stale file from a
/// previous run before binding:
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
    type Addr = tokio::net::unix::SocketAddr;

    async fn accept(&mut self) -> io::Result<(Self::Io, Self::Addr)> {
        UnixListener::accept(self).await
    }
}
