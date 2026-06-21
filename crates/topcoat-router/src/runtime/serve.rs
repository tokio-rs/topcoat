use std::{env, io};

use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::TcpListener;

use crate::runtime::{Router, RouterService};

/// Serves a [`Router`] on an already-bound [`TcpListener`].
///
/// Accepts connections in a loop, serving each on its own task, until the
/// listener errors.
pub async fn serve(listener: TcpListener, router: Router) -> io::Result<()> {
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

/// Binds to the configured host and port and serves `router`.
///
/// The listener binds to the `HOST` and `PORT` environment variables, or
/// `127.0.0.1` and `3000` when unset.
pub async fn start(router: Router) -> io::Result<()> {
    let host = host_from_env()?;
    let port = port_from_env()?;
    let listener = TcpListener::bind((host.as_str(), port)).await?;

    serve(listener, router).await
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
