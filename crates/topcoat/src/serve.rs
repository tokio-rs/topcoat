use std::{env, io};

use tokio::net::TcpListener;

/// Serve a Topcoat/Axum router, notifying the topcoat dev server once the
/// application is ready to accept connections.
///
/// This is a thin wrapper around [`axum::serve`] that calls
/// [`crate::dev::notify_ready`] before entering the accept loop.
pub async fn serve(
    listener: TcpListener,
    app: impl Into<axum::Router>,
) -> Result<(), std::io::Error> {
    crate::dev::notify_ready().await;
    axum::serve(listener, app.into()).await
}

/// Start a Topcoat/Axum router on the configured host and port.
///
/// The listener binds to the `HOST` and `PORT` environment variables,
/// or `127.0.0.1` and `3000` when unset.
pub async fn start(app: impl Into<axum::Router>) -> Result<(), io::Error> {
    let host = host_from_env()?;
    let port = port_from_env()?;
    let listener = TcpListener::bind((host.as_str(), port)).await?;

    serve(listener, app).await
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
