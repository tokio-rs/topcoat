//! The in-memory transport for tests.

use std::sync::{Arc, Mutex, PoisonError};

use topcoat_core::context::Cx;

use crate::{
    Mail, Receipt, Transport, TransportFuture,
    mime::{self, BccHeader},
};

/// A [`Transport`] that captures sent mail in memory instead of delivering
/// it, for asserting on mail in tests.
///
/// Sending assembles the mail exactly as a delivering transport would, so a
/// mail that would fail to send (one without recipients, say) fails here
/// too. Cloning is cheap and every clone shares the same capture, so a test
/// can keep one clone and hand the other to the code under test:
///
/// ```
/// # use topcoat_core::context::Cx;
/// use topcoat_mail::{Mail, MemoryTransport, Transport};
///
/// # async fn test() -> topcoat_core::error::Result<()> {
/// # let cx = Cx::default();
/// # let mail = Mail::builder()
/// #     .from(topcoat_mail::Mailbox::new("ada@example.com")?)
/// #     .to([topcoat_mail::Mailbox::new("bob@example.com")?])
/// #     .subject("Welcome!")
/// #     .text("Hi")
/// #     .build();
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
    fn send<'a>(&'a self, cx: &'a Cx, mail: Mail) -> TransportFuture<'a> {
        Box::pin(async move {
            let message = mime::message(cx, &mail, BccHeader::Dropped)?;
            let receipt = Receipt::new(mime::message_id(&message));
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
    }

    #[tokio::test]
    async fn captures_sent_mail_across_clones() {
        let transport = MemoryTransport::new();
        let clone = transport.clone();

        let receipt = clone.send(&Cx::default(), mail()).await.unwrap();
        assert!(receipt.message_id().contains('@'));

        let sent = transport.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject(), "Hello");

        transport.clear();
        assert!(clone.sent().is_empty());
    }

    #[tokio::test]
    async fn rejects_mail_that_could_not_be_sent() {
        let transport = MemoryTransport::new();
        let unsendable = Mail::builder().text("Hi").build();

        assert!(transport.send(&Cx::default(), unsendable).await.is_err());
        assert!(transport.sent().is_empty());
    }
}
