use std::future::Future;
use std::{env, io};

use tokio::net::TcpListener;

use crate::router::{RouterService, internal_serve};

/// Serve a Topcoat router, notifying the topcoat dev server once the
/// application is ready to accept connections.
///
/// The server runs until the process receives a shutdown signal: Ctrl+C, or
/// `SIGTERM` on Unix. It then shuts down gracefully, giving in-flight
/// requests the service's shutdown timeout to finish (see
/// [`RouterService::shutdown_timeout`]). To shut down on a custom signal
/// instead, use [`serve_until`].
///
/// This calls [`crate::dev::notify_ready`] before handing the listener off to
/// the router's accept loop.
///
/// # Errors
///
/// Returns `Err` if accepting a connection on `listener` fails.
pub async fn serve(
    listener: TcpListener,
    service: impl Into<RouterService>,
) -> Result<(), io::Error> {
    serve_until(listener, service, shutdown_signal()).await
}

/// Serve a Topcoat router until `signal` completes.
///
/// Like [`serve`], but shutting down when the given future resolves rather
/// than on a process signal. When `signal` completes, the server stops
/// accepting connections and gives in-flight requests the service's shutdown
/// timeout to finish (see [`RouterService::shutdown_timeout`]).
///
/// # Errors
///
/// Returns `Err` if accepting a connection on `listener` fails.
pub async fn serve_until(
    listener: TcpListener,
    service: impl Into<RouterService>,
    signal: impl Future<Output = ()>,
) -> Result<(), io::Error> {
    let addr = listener.local_addr().ok();
    crate::dev::notify_ready(addr).await;
    internal_serve(listener, service.into(), signal).await
}

/// Start a Topcoat router on the configured host and port.
///
/// The listener binds to the `HOST` and `PORT` environment variables,
/// or `127.0.0.1` and `3000` when unset.
///
/// The server runs until the process receives a shutdown signal: Ctrl+C, or
/// `SIGTERM` on Unix. It then shuts down gracefully, giving in-flight
/// requests the service's shutdown timeout to finish (see
/// [`RouterService::shutdown_timeout`]). To shut down on a custom signal
/// instead, use [`serve_until`].
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
        _ = terminate => {}
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
