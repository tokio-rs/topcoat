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

/// The boxed future returned by [`Transport::send`].
pub type TransportFuture<'a> = Pin<Box<dyn Future<Output = Result<Receipt>> + Send + 'a>>;

/// Delivers a [`Mail`] to its recipients.
///
/// A transport turns the mail into a message, rendering the HTML body with
/// the request context, and delivers it. Topcoat includes transports for
/// SMTP, for writing files during development, and for capturing mail in
/// tests.
///
/// Implement this trait to deliver mail some other way, such as through the
/// HTTP API of a mail provider. [`Mail::formatted`] renders a complete RFC
/// 5322 message for APIs that accept raw messages.
pub trait Transport: Send + Sync {
    /// Sends the mail and returns a [`Receipt`] once it is accepted for
    /// delivery.
    ///
    /// A receipt means that the mail was handed off, not that it reached an
    /// inbox.
    fn send<'a>(&'a self, cx: &'a Cx, mail: Mail) -> TransportFuture<'a>;
}

/// Confirmation that a transport accepted a mail, with the mail's
/// `Message-ID`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    message_id: String,
}

impl Receipt {
    /// Creates a receipt for the mail with the given `Message-ID`, for use in
    /// [`Transport`] implementations.
    #[must_use]
    pub fn new(message_id: impl Into<String>) -> Receipt {
        Receipt {
            message_id: message_id.into(),
        }
    }

    /// The `Message-ID` header of the sent mail. It is generated when the
    /// mail is sent, unless the mail set one.
    ///
    /// Save it to send a later mail in the same thread with
    /// [`in_reply_to`](crate::MailBuilder::in_reply_to).
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }
}

/// The error returned when a mail cannot be sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SendError {
    /// The mail has no `From` address.
    #[error("mail has no `From` address")]
    MissingFrom,

    /// The mail has no `To`, `Cc`, or `Bcc` recipient.
    #[error("mail has no recipients")]
    MissingRecipients,

    /// The mail has neither an HTML nor a plain-text body.
    #[error("mail has no body")]
    MissingBody,

    /// The mail has an inline attachment but no HTML body to show it in.
    #[error("mail has an inline attachment but no HTML body to reference it")]
    InlineWithoutHtml,

    /// An attachment has a content type that is not a valid MIME type.
    #[error("invalid attachment content type {content_type:?}")]
    InvalidContentType {
        /// The rejected content type.
        content_type: String,
    },

    /// A custom header has an invalid name.
    #[error("invalid header name {name:?}")]
    InvalidHeaderName {
        /// The rejected header name.
        name: String,
    },

    /// The mail could not be turned into a message.
    #[error("could not assemble the mail")]
    Assembly(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The transport failed to deliver the mail.
    #[error("could not deliver the mail")]
    Delivery(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl SendError {
    /// Creates a [`Delivery`](Self::Delivery) error from `error`, for use in
    /// [`Transport`] implementations.
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
