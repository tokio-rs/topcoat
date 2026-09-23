use std::{env, future::Future, io};

use tokio::net::TcpListener;

use crate::router::{Listener, RouterService, internal_serve};

/// Serves a Topcoat router and notifies the dev server when the
/// application is ready to accept connections.
///
/// Pass a bound [`Listener`]. For example, on Unix you can accept connections
/// through a socket behind a reverse proxy:
///
/// ```no_run
/// # #[cfg(unix)]
/// # async fn serve(router: topcoat::router::Router) -> std::io::Result<()> {
/// let path = "/run/my-app.sock";
/// let _ = std::fs::remove_file(path);
/// let listener = tokio::net::UnixListener::bind(path)?;
/// topcoat::serve(listener, router).await
/// # }
/// ```
///
/// Ctrl+C, or `SIGTERM` on Unix, stops new connections. Active requests have
/// until [`RouterService::shutdown_timeout`] to finish. Use [`serve_until`]
/// to choose when shutdown starts.
///
/// # Errors
///
/// Returns `Err` if accepting a connection on `listener` fails.
pub async fn serve(
    listener: impl Listener,
    service: impl Into<RouterService>,
) -> Result<(), io::Error> {
    serve_until(listener, service, shutdown_signal()).await
}

/// Serves a Topcoat router until `signal` completes.
///
/// Notifies the dev server when ready. When `signal` completes, the server
/// stops accepting connections and gives active requests until
/// [`RouterService::shutdown_timeout`] to finish.
///
/// # Errors
///
/// Returns `Err` if accepting a connection on `listener` fails.
pub async fn serve_until(
    listener: impl Listener,
    service: impl Into<RouterService>,
    signal: impl Future<Output = ()>,
) -> Result<(), io::Error> {
    let addr = listener.tcp_addr();
    crate::dev::notify_ready(addr).await;
    internal_serve(listener, service.into(), signal).await
}

/// Starts a Topcoat router on the configured host and port.
///
/// The listener binds to the `HOST` and `PORT` environment variables,
/// or `127.0.0.1` and `3000` when unset.
///
/// Uses [`serve`] to handle connections and graceful shutdown.
///
/// # Errors
///
/// Returns `Err` if `HOST`/`PORT` are invalid, if binding the TCP listener
/// fails, or if serving the router fails (see [`serve`]).
pub async fn start(service: impl Into<RouterService>) -> Result<(), io::Error> {
    let host = host_from_env()?;
    let port = port_from_env()?;
    let listener = TcpListener::bind((host.as_str(), port)).await?;

    serve(listener, service).await
}

/// Resolves when the process receives a shutdown signal: Ctrl+C, or `SIGTERM`
/// on Unix.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install the Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install the SIGTERM signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

fn host_from_env() -> Result<String, io::Error> {
    const HOST_ENV: &str = "HOST";
    const DEFAULT_HOST: &str = "127.0.0.1";

    match env::var(HOST_ENV) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_HOST.to_owned()),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{HOST_ENV} must be valid Unicode: {error}"),
        )),
    }
}

fn port_from_env() -> Result<u16, io::Error> {
    const PORT_ENV: &str = "PORT";
    const DEFAULT_PORT: u16 = 3000;

    match env::var(PORT_ENV) {
        Ok(value) => value.parse().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{PORT_ENV} must be a valid port number: {error}"),
            )
        }),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_PORT),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{PORT_ENV} must be valid Unicode: {error}"),
        )),
    }
}
