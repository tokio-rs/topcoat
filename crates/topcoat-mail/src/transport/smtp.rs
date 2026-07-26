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

/// A [`Transport`] that submits mail to an SMTP server.
///
/// This is the production transport: point it at a mail provider's
/// submission endpoint or your own mail server. Connections are pooled and
/// reused across sends.
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
    /// Connects to `host` over implicit TLS on port 465, the modern
    /// submission setup most mail providers offer.
    ///
    /// # Errors
    ///
    /// Returns [`SmtpError`] if the TLS parameters for `host` cannot be
    /// established.
    pub fn relay(host: &str) -> Result<SmtpTransportBuilder, SmtpError> {
        Ok(SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(SmtpError)?,
        })
    }

    /// Connects to `host` on port 587, upgrading the connection with
    /// STARTTLS, for providers that only offer the older submission setup.
    ///
    /// # Errors
    ///
    /// Returns [`SmtpError`] if the TLS parameters for `host` cannot be
    /// established.
    pub fn starttls(host: &str) -> Result<SmtpTransportBuilder, SmtpError> {
        Ok(SmtpTransportBuilder {
            inner: AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host).map_err(SmtpError)?,
        })
    }

    /// Connects to `host` on port 25 without encryption.
    ///
    /// Credentials and mail travel in clear text, so this belongs only in
    /// development, against a local catch-all server like Mailpit:
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

    /// Connects according to a connection URL, the form that fits a single
    /// environment variable:
    ///
    /// - `smtps://user:pass@smtp.example.com:465` for implicit TLS.
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
            let message = mime::message(cx, &mail, BccHeader::Dropped)?;
            let message_id = mime::message_id(&message);
            self.inner
                .send(message)
                .await
                .map_err(SendError::delivery)?;
            Ok(Receipt::new(message_id))
        })
    }
}

/// Assembles an [`SmtpTransport`], created by its connection constructors.
pub struct SmtpTransportBuilder {
    inner: AsyncSmtpTransportBuilder,
}

impl SmtpTransportBuilder {
    /// Overrides the port the connection constructor chose.
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

    /// Finishes the builder into an [`SmtpTransport`].
    #[must_use]
    pub fn build(self) -> SmtpTransport {
        SmtpTransport {
            inner: self.inner.build(),
        }
    }
}

/// The reason an SMTP connection could not be configured.
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
