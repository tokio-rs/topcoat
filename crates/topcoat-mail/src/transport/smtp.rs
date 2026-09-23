//! The SMTP transport.

use std::time::Duration;

use lettre::{
    AsyncSmtpTransport, AsyncTransport as _, Tokio1Executor,
    transport::smtp::{AsyncSmtpTransportBuilder, authentication::Credentials},
};
use topcoat_core::context::Cx;

use crate::{
    Mail, Receipt, SendError, Transport, TransportFuture,
    mime::{self, BccHeader},
};

/// A [`Transport`] that sends mail to an SMTP server.
///
/// Use it in production with the submission server of a mail provider or with
/// your own mail server. It keeps a pool of connections and reuses them
/// across sends.
///
/// ```no_run
/// use topcoat_mail::SmtpTransport;
///
/// # fn main() -> Result<(), topcoat_mail::SmtpError> {
/// let transport = SmtpTransport::relay("smtp.example.com")?
///     .credentials("username", "password")
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct SmtpTransport {
    inner: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpTransport {
    /// Configures a connection to `host` over TLS on port 465, which most
    /// mail providers offer.
    ///
    /// # Errors
    ///
    /// Returns [`SmtpError`] if the TLS settings for `host` cannot be
    /// created.
    pub fn relay(host: &str) -> Result<SmtpTransportBuilder, SmtpError> {
        Ok(SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(SmtpError)?,
        })
    }

    /// Configures a connection to `host` on port 587 that is upgraded to TLS
    /// with STARTTLS, for providers that do not offer port 465.
    ///
    /// # Errors
    ///
    /// Returns [`SmtpError`] if the TLS settings for `host` cannot be
    /// created.
    pub fn starttls(host: &str) -> Result<SmtpTransportBuilder, SmtpError> {
        Ok(SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host).map_err(SmtpError)?,
        })
    }

    /// Configures a connection to `host` on port 25 without encryption.
    ///
    /// Credentials and mail are sent as plain text, so only use this in
    /// development, with a local test server such as Mailpit:
    ///
    /// ```no_run
    /// use topcoat_mail::SmtpTransport;
    ///
    /// let transport = SmtpTransport::unencrypted("localhost").port(1025).build();
    /// ```
    #[must_use]
    pub fn unencrypted(host: &str) -> SmtpTransportBuilder {
        SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
        }
    }

    /// Configures a connection from a URL, which fits well into a single
    /// environment variable:
    ///
    /// - `smtps://user:pass@smtp.example.com:465` for TLS.
    /// - `smtp://user:pass@smtp.example.com:587?tls=required` for STARTTLS.
    /// - `smtp://localhost:1025` for an unencrypted development server.
    ///
    /// Credentials must be URL-encoded.
    ///
    /// # Errors
    ///
    /// Returns [`SmtpError`] if the URL is not a valid SMTP connection URL.
    pub fn from_url(url: &str) -> Result<SmtpTransportBuilder, SmtpError> {
        Ok(SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::from_url(url).map_err(SmtpError)?,
        })
    }
}

impl Transport for SmtpTransport {
    fn send<'a>(&'a self, cx: &'a Cx, mail: Mail) -> TransportFuture<'a> {
        Box::pin(async move {
            let message = mime::message(cx, mail, BccHeader::Dropped)?;
            let message_id = mime::message_id(&message);
            self.inner
                .send(message)
                .await
                .map_err(SendError::delivery)?;
            Ok(Receipt::new(message_id))
        })
    }
}

/// Builder for an [`SmtpTransport`], created by [`SmtpTransport::relay`] and
/// the other constructors.
pub struct SmtpTransportBuilder {
    inner: AsyncSmtpTransportBuilder,
}

impl SmtpTransportBuilder {
    /// Sets the port, replacing the one the constructor chose.
    #[must_use]
    pub fn port(mut self, port: u16) -> SmtpTransportBuilder {
        self.inner = self.inner.port(port);
        self
    }

    /// Authenticates with the given username and password.
    #[must_use]
    pub fn credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> SmtpTransportBuilder {
        self.inner = self
            .inner
            .credentials(Credentials::new(username.into(), password.into()));
        self
    }

    /// Sets the connection timeout. The default is 60 seconds.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> SmtpTransportBuilder {
        self.inner = self.inner.timeout(Some(timeout));
        self
    }

    /// Returns the finished [`SmtpTransport`].
    #[must_use]
    pub fn build(self) -> SmtpTransport {
        SmtpTransport {
            inner: self.inner.build(),
        }
    }
}

/// The error returned when an SMTP connection cannot be configured.
#[derive(Debug, thiserror::Error)]
#[error("invalid SMTP configuration: {0}")]
pub struct SmtpError(lettre::transport::smtp::Error);

#[cfg(test)]
mod tests {
    use super::*;

    // The connection pool needs a runtime to wind down when the transport
    // drops.
    #[tokio::test]
    async fn constructors_accept_their_configuration() {
        let _ = SmtpTransport::relay("smtp.example.com")
            .unwrap()
            .credentials("user", "pass")
            .timeout(Duration::from_secs(5))
            .build();
        let _ = SmtpTransport::starttls("smtp.example.com").unwrap().build();
        let _ = SmtpTransport::unencrypted("localhost").port(1025).build();
        let _ = SmtpTransport::from_url("smtps://user:pass@smtp.example.com:465")
            .unwrap()
            .build();
    }

    #[test]
    fn rejects_an_invalid_connection_url() {
        assert!(SmtpTransport::from_url("https://example.com").is_err());
    }
}
