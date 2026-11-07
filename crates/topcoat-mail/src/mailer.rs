//! Sending mail through the transport in the context.

use topcoat_core::{
    context::{Cx, context},
    error::Result,
};

use crate::{Mail, Receipt, Transport};

/// Delivers mail through a [`Transport`].
///
/// Add a mailer to the context so that [`send`] uses its transport:
///
/// ```
/// use topcoat_core::context::Cx;
/// use topcoat_mail::{Mail, Mailbox, Mailer, MemoryTransport, send};
///
/// # async fn example() -> topcoat_core::error::Result<()> {
/// let cx = Cx::new().with(Mailer::new(MemoryTransport::new()));
///
/// let mail = Mail::builder()
///     .from(Mailbox::new("ada@example.com")?)
///     .to([Mailbox::new("bob@example.com")?])
///     .subject("Analytical engines")
///     .text("The engine weaves algebraic patterns.")
///     .build()?;
///
/// let receipt = send(&cx, mail).await?;
/// # Ok(())
/// # }
/// ```
pub struct Mailer {
    transport: Box<dyn Transport>,
}

impl Mailer {
    /// Creates a mailer that delivers through `transport`.
    #[must_use]
    pub fn new(transport: impl Transport + 'static) -> Mailer {
        Mailer {
            transport: Box::new(transport),
        }
    }
}

/// Sends a message through the [`Mailer`] in the context. Returns a
/// [`Receipt`] when the transport accepts it.
///
/// # Panics
///
/// Panics if the context has no [`Mailer`].
///
/// # Errors
///
/// Returns an error if the message cannot be formatted or delivered.
/// See [`SendError`](crate::SendError).
pub async fn send(cx: &Cx, mail: Mail) -> Result<Receipt> {
    let mailer: &Mailer = context(cx);
    mailer.transport.send(cx, mail).await
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{Mailbox, MemoryTransport};

    fn mail() -> Result<Mail> {
        Ok(Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .subject("Hello")
            .text("Hi")
            .build()?)
    }

    #[tokio::test]
    async fn sends_through_the_mailer_transport() -> Result<()> {
        let transport = MemoryTransport::new();
        let cx = Cx::new().with(Mailer::new(transport.clone()));

        let receipt = send(&cx, mail()?).await?;

        assert_ne!(receipt.message_id(), "");
        assert_eq!(transport.sent().len(), 1);
        assert_eq!(transport.sent()[0].subject(), "Hello");

        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "attempted to access context")]
    async fn send_panics_without_a_mailer() {
        let cx = Cx::new();
        let _ = send(&cx, mail().unwrap()).await;
    }
}
