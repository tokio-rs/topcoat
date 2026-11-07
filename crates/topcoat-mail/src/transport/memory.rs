//! The in-memory transport for tests.

use std::sync::{Arc, Mutex, PoisonError};

use topcoat_core::context::Cx;

use crate::{Mail, Receipt, Transport, TransportFuture};

/// A [`Transport`] that captures messages in memory for tests.
///
/// Clones share the same capture, so a test can keep one clone and give
/// another to the application:
///
/// ```
/// # use topcoat_core::context::Cx;
/// use topcoat_mail::{Mail, MemoryTransport, Transport};
///
/// # async fn test() -> topcoat_core::error::Result<()> {
/// # let cx = Cx::new();
/// # let mail = Mail::builder()
/// #     .from(topcoat_mail::Mailbox::new("ada@example.com")?)
/// #     .to([topcoat_mail::Mailbox::new("bob@example.com")?])
/// #     .subject("Welcome!")
/// #     .text("Hi")
/// #     .build()?;
/// let transport = MemoryTransport::new();
///
/// transport.send(&cx, mail).await?;
///
/// let sent = transport.sent();
/// assert_eq!(sent.len(), 1);
/// assert_eq!(sent[0].subject(), "Welcome!");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct MemoryTransport {
    sent: Arc<Mutex<Vec<Mail>>>,
}

impl MemoryTransport {
    /// Creates a transport with an empty capture.
    #[must_use]
    pub fn new() -> MemoryTransport {
        MemoryTransport::default()
    }

    /// The captured mail, in the order it was sent.
    #[must_use]
    pub fn sent(&self) -> Vec<Mail> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Empties the capture.
    pub fn clear(&self) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

impl Transport for MemoryTransport {
    fn send<'a>(&'a self, _cx: &'a Cx, mail: Mail) -> TransportFuture<'a> {
        Box::pin(async move {
            let receipt = Receipt::new(mail.message_id());
            self.sent
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(mail);
            Ok(receipt)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mailbox;

    fn mail() -> Mail {
        Mail::builder()
            .from(Mailbox::new("ada@example.com").unwrap())
            .to([Mailbox::new("bob@example.com").unwrap()])
            .subject("Hello")
            .text("Hi Bob")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn captures_sent_mail_across_clones() {
        let transport = MemoryTransport::new();
        let clone = transport.clone();

        let mail = mail();
        let receipt = clone.send(&Cx::new(), mail.clone()).await.unwrap();
        assert_eq!(receipt.message_id(), mail.message_id());

        let sent = transport.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject(), "Hello");

        transport.clear();
        assert_eq!(clone.sent(), []);
    }
}
