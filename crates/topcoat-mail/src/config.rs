//! Mail configuration and sending through the configured transport.

use topcoat_core::{
    context::{Cx, app_context},
    error::Result,
};

use crate::{Mail, Receipt, Transport};

/// The transport used to send application mail.
///
/// Choose a [`Transport`] with [`MailConfig::builder`] and register the
/// configuration in app context. Handlers can then call [`send`] without
/// choosing a transport:
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

/// Assembles a [`MailConfig`]. Created with [`MailConfig::builder`].
#[derive(Default)]
pub struct MailConfigBuilder {
    transport: Option<Box<dyn Transport>>,
}

impl MailConfigBuilder {
    /// Sets the [`Transport`] the application delivers mail through.
    #[must_use]
    pub fn transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    /// Consumes the builder, returning the finished [`MailConfig`].
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

/// Sends a message through the registered [`MailConfig`]. Returns a [`Receipt`]
/// when the transport accepts it.
///
/// # Panics
///
/// Panics when no [`MailConfig`] is registered on the app context.
///
/// # Errors
///
/// Returns an error if the message cannot be formatted or delivered.
/// See [`SendError`](crate::SendError).
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
