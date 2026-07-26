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
/// A transport receives the mail as declared content, assembles its wire
/// form (rendering the HTML view with the request context), and hands it
/// to a delivery mechanism. The crate ships three: `SmtpTransport` submits
/// to an SMTP server (behind the `smtp` feature), [`FileTransport`] writes
/// `.eml` files during development, and [`MemoryTransport`] captures mail in
/// tests.
///
/// Implement this trait to deliver through anything else, such as a mail
/// provider's HTTP API. [`Mail::formatted`] produces the RFC 5322 wire form
/// for APIs that accept raw messages.
pub trait Transport: Send + Sync {
    /// Sends the mail, returning a [`Receipt`] once the delivery mechanism
    /// accepts it.
    ///
    /// Acceptance is not receipt: a transport reports that the mail was
    /// handed off successfully, not that it reached an inbox.
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
    /// Store it to thread a later mail onto this one with
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
    /// The mail declares no `From` address.
    #[error("mail has no `From` address")]
    MissingFrom,

    /// The mail declares no `To`, `Cc`, or `Bcc` recipient.
    #[error("mail has no recipients")]
    MissingRecipients,

    /// The mail declares neither an HTML nor a plain-text body.
    #[error("mail has no body")]
    MissingBody,

    /// The mail carries an inline attachment but no HTML body that could
    /// reference it.
    #[error("mail has an inline attachment but no HTML body to reference it")]
    InlineWithoutHtml,

    /// An attachment declares a content type that is not a valid MIME type.
    #[error("invalid attachment content type {content_type:?}")]
    InvalidContentType {
        /// The rejected content type.
        content_type: String,
    },

    /// A custom header declares an invalid header name.
    #[error("invalid header name {name:?}")]
    InvalidHeaderName {
        /// The rejected header name.
        name: String,
    },

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
