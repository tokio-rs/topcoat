use std::io;

use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;

use crate::runtime::{Router, RouterService};

/// Serves a [`Router`] on an already-bound [`TcpListener`].
///
/// This is the low-level accept loop, with no dev-server integration: it
/// accepts connections in a loop, serving each on its own task, until the
/// listener errors. Applications typically use the facade's `serve`/`start`
/// helpers, which layer dev-server readiness notification on top of this.
///
/// # Errors
///
/// Returns an I/O error if accepting a connection fails.
pub async fn internal_serve(listener: TcpListener, router: Router) -> io::Result<()> {
    let service = RouterService::new(router);

    loop {
        let (stream, _remote) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let service = service.clone();

        tokio::spawn(async move {
            if let Err(error) = auto::Builder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                eprintln!("error serving connection: {error}");
            }
        });
    }
}
