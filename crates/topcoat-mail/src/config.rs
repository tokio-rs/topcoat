//! Mail configuration and sending through the configured transport.

use topcoat_core::{
    context::{Cx, app_context},
    error::Result,
};

use crate::{Mail, Receipt, Transport};

/// Mail configuration: the [`Transport`] the application sends mail through.
///
/// Build one with [`MailConfig::builder`] and register it as app context,
/// usually with `.mail(config)` on the router builder. Code then sends mail
/// with [`send`] without naming the transport, so you can switch transports,
/// for example between development and production, without changing that
/// code:
///
/// ```
/// use topcoat_core::context::CxTestBuilder;
/// use topcoat_mail::{Mail, MailConfig, Mailbox, MemoryTransport, send};
///
/// # async fn example() -> topcoat_core::error::Result<()> {
/// let config = MailConfig::builder()
///     .transport(MemoryTransport::new())
///     .build();
/// let cx = CxTestBuilder::new().app_context(config).build();
///
/// let mail = Mail::builder()
///     .from(Mailbox::new("ada@example.com")?)
///     .to([Mailbox::new("bob@example.com")?])
///     .subject("Analytical engines")
///     .text("The engine weaves algebraic patterns.")
///     .build();
///
/// let receipt = send(&cx, mail).await?;
/// # Ok(())
/// # }
/// ```
pub struct MailConfig {
    pub(crate) transport: Box<dyn Transport>,
}

impl MailConfig {
    /// Creates a builder for a mail configuration.
    #[must_use]
    pub fn builder() -> MailConfigBuilder {
        MailConfigBuilder::default()
    }
}

/// Builder for a [`MailConfig`], created with [`MailConfig::builder`].
#[derive(Default)]
pub struct MailConfigBuilder {
    transport: Option<Box<dyn Transport>>,
}

impl MailConfigBuilder {
    /// Sets the [`Transport`] the application sends mail through.
    #[must_use]
    pub fn transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    /// Returns the finished [`MailConfig`].
    ///
    /// # Panics
    ///
    /// Panics when no transport was set.
    #[must_use]
    #[track_caller]
    pub fn build(self) -> MailConfig {
        MailConfig {
            transport: self.transport.unwrap_or_else(|| {
                panic!("no transport configured: set one with `MailConfigBuilder::transport`")
            }),
        }
    }
}

/// Sends `mail` through the transport of the [`MailConfig`] registered as app
/// context.
///
/// Returns a [`Receipt`] once the transport accepts the mail. This does not
/// mean the mail has reached the recipient's inbox.
///
/// # Panics
///
/// Panics when no [`MailConfig`] is registered on the app context.
///
/// # Errors
///
/// Returns an error when the mail is incomplete or invalid, or when the
/// transport fails to deliver it. See [`SendError`](crate::SendError).
pub async fn send(cx: &Cx, mail: Mail) -> Result<Receipt> {
    let config: &MailConfig = app_context(cx);
    config.transport.send(cx, mail).await
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::CxTestBuilder;

    use super::*;
    use crate::{AddressError, Mailbox, MemoryTransport};

    fn mail() -> Result<Mail, AddressError> {
        Ok(Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .subject("Hello")
            .text("Hi")
            .build())
    }

    fn cx_with(transport: MemoryTransport) -> Cx {
        CxTestBuilder::new()
            .app_context(MailConfig::builder().transport(transport).build())
            .build()
    }

    #[tokio::test]
    async fn sends_through_the_configured_transport() -> topcoat_core::error::Result<()> {
        let transport = MemoryTransport::new();
        let cx = cx_with(transport.clone());

        let receipt = send(&cx, mail()?).await?;

        assert!(!receipt.message_id().is_empty());
        assert_eq!(transport.sent().len(), 1);
        assert_eq!(transport.sent()[0].subject(), "Hello");

        Ok(())
    }

    #[tokio::test]
    async fn propagates_transport_errors() {
        let cx = cx_with(MemoryTransport::new());

        // No recipients, so assembly fails before anything is captured.
        let result = send(&cx, Mail::builder().build()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    #[should_panic(expected = "app context")]
    async fn send_panics_without_a_registered_config() {
        let cx = Cx::default();
        let _ = send(&cx, Mail::builder().build()).await;
    }

    #[test]
    #[should_panic(expected = "no transport configured")]
    fn build_panics_without_a_transport() {
        let _ = MailConfig::builder().build();
    }
}
