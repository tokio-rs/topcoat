use std::{env, io, net::TcpListener};

use console::style;

use super::keyboard::Keyboard;

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
/// `HOST`/`PORT`.
pub async fn resolve(keyboard: &mut Keyboard) -> Option<Address> {
    let mut address = Address::from_env();
    let original_port = address.port;

    loop {
        match TcpListener::bind((address.host.as_str(), address.port)) {
            Ok(_listener) => break,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                address.port = address.port.checked_add(1)?;
            }
            Err(_) => break,
        }
    }

    if address.port == original_port {
        return Some(address);
    }

    let question = style(format!(
        "  Port {original_port} is already in use. Use port {} instead? [",
        address.port
    ))
    .for_stderr()
    .dim();
    let yes = style("Y").for_stderr().green().bold();
    let slash = style("/").for_stderr().dim();
    let no = style("n").for_stderr().red().bold();
    let suffix = style("] ").for_stderr().dim();
    let message = format!("{question}{yes}{slash}{no}{suffix}");

    if !keyboard.confirm(&message).await {
        return None;
    }

    Some(address)
}
