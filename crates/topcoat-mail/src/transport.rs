//! Delivering mail through a transport.

use std::pin::Pin;

use topcoat_core::{context::Cx, error::Result};

use crate::Mail;

mod file;
mod memory;
#[cfg(feature = "smtp")]
mod smtp;

pub use file::*;
pub use memory::*;
#[cfg(feature = "smtp")]
pub use smtp::*;

/// The future returned by [`Transport::send`]: a boxed, `Send` future
/// borrowing the transport and the request context.
pub type TransportFuture<'a> = Pin<Box<dyn Future<Output = Result<Receipt>> + Send + 'a>>;

/// Delivers a [`Mail`] to its recipients.
///
/// Implement this trait to integrate a delivery service. The request context
/// is available when rendering the HTML body. Use [`Mail::formatted`] for
/// services that accept a complete RFC 5322 message.
pub trait Transport: Send + Sync {
    /// Sends the mail, returning a [`Receipt`] once the delivery mechanism
    /// accepts it.
    ///
    /// A successful send confirms that the transport accepted the message.
    /// It does not confirm inbox delivery.
    fn send<'a>(&'a self, cx: &'a Cx, mail: Mail) -> TransportFuture<'a>;
}

/// Proof that a transport accepted a mail, carrying its `Message-ID`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    message_id: String,
}

impl Receipt {
    /// A receipt for the mail with the given `Message-ID`. Called by
    /// [`Transport`] implementations.
    #[must_use]
    pub fn new(message_id: impl Into<String>) -> Receipt {
        Receipt {
            message_id: message_id.into(),
        }
    }

    /// The `Message-ID` of the sent mail, as it appears in the header.
    /// It is generated at send time unless the mail declared one.
    ///
    /// Use it to link a later reply with
    /// [`in_reply_to`](crate::MailBuilder::in_reply_to).
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
}

/// The reason a mail could not be sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SendError {
    /// The mail could not be assembled into its wire form.
    #[error("could not assemble the mail")]
    Assembly(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The delivery mechanism failed to accept the mail.
    #[error("could not deliver the mail")]
    Delivery(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl SendError {
    /// Wraps a delivery failure. Called by [`Transport`] implementations
    /// to report errors from the underlying delivery mechanism.
    #[must_use]
    pub fn delivery(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> SendError {
        SendError::Delivery(error.into())
    }

    /// Wraps an assembly failure.
    pub(crate) fn assembly(
        error: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> SendError {
        SendError::Assembly(error.into())
    }
}
