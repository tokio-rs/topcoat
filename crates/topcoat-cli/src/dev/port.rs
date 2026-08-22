use std::{env, io, net::TcpListener};

use console::style;

/// Mirrors the defaults `topcoat::serve::start` applies when `HOST`/`PORT`
/// are unset.
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3000;

/// The host and port the application should bind.
pub(crate) struct Address {
    pub host: String,
    pub port: u16,
}

impl Address {
    fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned()),
            port: env::var("PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_PORT),
        }
    }
}

/// Find a host and port for the application to bind, starting from
/// `HOST`/`PORT` and walking up from an occupied port to the first free one.
/// Picking a different port than the configured one is reported on the
/// terminal.
///
/// Returns `None` when every port from the configured one upward is occupied.
pub fn resolve() -> Option<Address> {
    let mut address = Address::from_env();
    let original_port = address.port;

    loop {
        match TcpListener::bind((address.host.as_str(), address.port)) {
            Ok(_listener) => break,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                address.port = address.port.checked_add(1)?;
            }
            // Any other error (an unresolvable host, a privileged port) is
            // left for the application to run into and report itself.
            Err(_) => break,
        }
    }

    if address.port != original_port {
        eprintln!(
            "  {}",
            style(format!(
                "port {original_port} is in use; starting on port {} instead",
                address.port
            ))
            .yellow()
        );
        eprintln!();
    }

    Some(address)
}
