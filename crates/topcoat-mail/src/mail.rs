//! Mail messages and the builder that assembles them.

use std::time::SystemTime;

use lettre::message::header::{ContentType, HeaderName};
use topcoat_view::ViewHandle;

use crate::{Attachment, Mailbox};

/// An email message ready to be formatted or sent.
///
/// Create one with [`Mail::builder`]. A mail always has a sender, at least
/// one recipient, and a body. Building a message does not send it:
///
/// ```
/// use topcoat_mail::{Mail, Mailbox};
///
/// let mail = Mail::builder()
///     .from(Mailbox::new("charles@example.com")?)
///     .to([Mailbox::named("Ada Lovelace", "ada@example.com")?])
///     .subject("Analytical engines")
///     .text("The engine weaves algebraic patterns.")
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Mail {
    from: Mailbox,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    reply_to: Vec<Mailbox>,
    subject: String,
    pub(crate) html: Option<ViewHandle>,
    text: TextBody,
    attachments: Vec<Attachment>,
    in_reply_to: Option<String>,
    references: Option<String>,
    headers: Vec<(String, String)>,
    date: Option<SystemTime>,
    message_id: Option<String>,
}

impl Mail {
    /// Starts building a mail.
    #[must_use]
    pub fn builder() -> MailBuilder {
        MailBuilder::default()
    }

    /// The `From` address.
    #[must_use]
    pub fn from(&self) -> &Mailbox {
        &self.from
    }

    /// The `To` recipients.
    #[must_use]
    pub fn to(&self) -> &[Mailbox] {
        &self.to
    }

    /// The `Cc` recipients.
    #[must_use]
    pub fn cc(&self) -> &[Mailbox] {
        &self.cc
    }

    /// The `Bcc` recipients.
    #[must_use]
    pub fn bcc(&self) -> &[Mailbox] {
        &self.bcc
    }

    /// The `Reply-To` addresses.
    #[must_use]
    pub fn reply_to(&self) -> &[Mailbox] {
        &self.reply_to
    }

    /// The subject line.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The HTML body, if any.
    #[must_use]
    pub fn html(&self) -> Option<&ViewHandle> {
        self.html.as_ref()
    }

    /// The plain-text body: derived, declared, or absent.
    #[must_use]
    pub fn text(&self) -> &TextBody {
        &self.text
    }

    /// The attachments, downloadable and inline alike.
    #[must_use]
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// The message id of the mail this one replies to, if any.
    #[must_use]
    pub fn in_reply_to(&self) -> Option<&str> {
        self.in_reply_to.as_deref()
    }

    /// The `References` header value, if any.
    #[must_use]
    pub fn references(&self) -> Option<&str> {
        self.references.as_deref()
    }

    /// Custom headers as `(name, value)` pairs.
    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// The `Date` header value, if one was set explicitly.
    #[must_use]
    pub fn date(&self) -> Option<SystemTime> {
        self.date
    }

    /// The `Message-ID`, if one was set explicitly.
    #[must_use]
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }
}

/// Assembles a [`Mail`], created by [`Mail::builder`].
///
/// The sender must convert into a [`Mailbox`]. Recipient setters accept
/// collections that convert into `Vec<Mailbox>` and append on each call.
/// Attachment and header setters also append.
///
/// [`build`](MailBuilder::build) checks that the mail is complete and that
/// its attachments and headers are valid.
#[derive(Clone, Debug, Default)]
pub struct MailBuilder {
    from: Option<Mailbox>,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    reply_to: Vec<Mailbox>,
    subject: String,
    html: Option<ViewHandle>,
    text: TextBody,
    attachments: Vec<Attachment>,
    in_reply_to: Option<String>,
    references: Option<String>,
    headers: Vec<(String, String)>,
    date: Option<SystemTime>,
    message_id: Option<String>,
}

impl MailBuilder {
    /// Sets the `From` address.
    #[must_use]
    pub fn from(mut self, from: impl Into<Mailbox>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Adds `To` recipients.
    #[must_use]
    pub fn to(mut self, to: impl Into<Vec<Mailbox>>) -> Self {
        self.to.extend(to.into());
        self
    }

    /// Adds `Cc` recipients.
    #[must_use]
    pub fn cc(mut self, cc: impl Into<Vec<Mailbox>>) -> Self {
        self.cc.extend(cc.into());
        self
    }

    /// Adds `Bcc` recipients, who receive the mail without appearing in
    /// its headers.
    #[must_use]
    pub fn bcc(mut self, bcc: impl Into<Vec<Mailbox>>) -> Self {
        self.bcc.extend(bcc.into());
        self
    }

    /// Adds `Reply-To` addresses, where replies are directed instead of the
    /// `From` address.
    #[must_use]
    pub fn reply_to(mut self, reply_to: impl Into<Vec<Mailbox>>) -> Self {
        self.reply_to.extend(reply_to.into());
        self
    }

    /// Sets the subject line.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Sets the HTML body.
    #[must_use]
    pub fn html(mut self, html: ViewHandle) -> Self {
        self.html = Some(html);
        self
    }

    /// Sets the plain-text body, the fallback for clients that do not
    /// render HTML.
    ///
    /// Accepts the text itself or a [`TextBody`] variant. Without a call,
    /// the text is derived from the HTML body when the mail is assembled;
    /// pass [`TextBody::None`] to send the HTML alone.
    #[must_use]
    pub fn text(mut self, text: impl Into<TextBody>) -> Self {
        self.text = text.into();
        self
    }

    /// Adds attachments.
    #[must_use]
    pub fn attachments(mut self, attachments: impl Into<Vec<Attachment>>) -> Self {
        self.attachments.extend(attachments.into());
        self
    }

    /// Adds custom `(name, value)` headers, such as `List-Unsubscribe`.
    #[must_use]
    pub fn headers(mut self, headers: impl Into<Vec<(String, String)>>) -> Self {
        self.headers.extend(headers.into());
        self
    }

    /// Marks the mail as a reply to the given message id.
    #[must_use]
    pub fn in_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.in_reply_to = Some(message_id.into());
        self
    }

    /// Sets the `References` header linking the message ids of the thread.
    #[must_use]
    pub fn references(mut self, references: impl Into<String>) -> Self {
        self.references = Some(references.into());
        self
    }

    /// Sets the `Date` header. Without an explicit date, the send time is
    /// used.
    #[must_use]
    pub fn date(mut self, date: SystemTime) -> Self {
        self.date = Some(date);
        self
    }

    /// Sets the `Message-ID`. Without an explicit id, one is generated at
    /// send time.
    #[must_use]
    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Finishes the builder into a [`Mail`].
    ///
    /// # Errors
    ///
    /// Returns [`MailError`] if the mail has no sender, no recipients, or
    /// no body, or if an attachment or header is invalid.
    pub fn build(self) -> Result<Mail, MailError> {
        let from = self.from.ok_or(MailError::MissingFrom)?;
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(MailError::MissingRecipients);
        }
        if self.html.is_none() {
            if !matches!(self.text, TextBody::Text(_)) {
                return Err(MailError::MissingBody);
            }
            if self
                .attachments
                .iter()
                .any(|attachment| attachment.content_id().is_some())
            {
                return Err(MailError::InlineWithoutHtml);
            }
        }
        for attachment in &self.attachments {
            if ContentType::parse(attachment.content_type()).is_err() {
                return Err(MailError::InvalidContentType {
                    content_type: attachment.content_type().to_owned(),
                });
            }
        }
        for (name, _) in &self.headers {
            if HeaderName::new_from_ascii(name.clone()).is_err() {
                return Err(MailError::InvalidHeaderName { name: name.clone() });
            }
        }

        Ok(Mail {
            from,
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            reply_to: self.reply_to,
            subject: self.subject,
            html: self.html,
            text: self.text,
            attachments: self.attachments,
            in_reply_to: self.in_reply_to,
            references: self.references,
            headers: self.headers,
            date: self.date,
            message_id: self.message_id,
        })
    }
}

/// The reason a [`MailBuilder`] could not build a [`Mail`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MailError {
    /// The mail declares no `From` address.
    #[error("mail has no `From` address")]
    MissingFrom,

    /// The mail declares no `To`, `Cc`, or `Bcc` recipient.
    #[error("mail has no recipients")]
    MissingRecipients,

    /// The mail declares neither an HTML body nor a plain-text body.
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
}

/// The plain-text body of a mail: derived, declared, or absent.
///
/// The default, [`FromHtml`](TextBody::FromHtml), derives text from the rendered
/// HTML body. Set your own text or omit the text body with
/// [`MailBuilder::text`]:
///
/// ```
/// use topcoat_mail::{Mail, TextBody};
///
/// let declared = Mail::builder().text("The engine weaves.");
/// let html_alone = Mail::builder().text(TextBody::None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TextBody {
    /// Derive the text from the HTML body when the mail is assembled.
    #[default]
    FromHtml,
    /// Send no plain-text body.
    None,
    /// Send the declared text.
    Text(String),
}

impl From<String> for TextBody {
    fn from(text: String) -> TextBody {
        TextBody::Text(text)
    }
}

impl From<&str> for TextBody {
    fn from(text: &str) -> TextBody {
        TextBody::Text(text.to_owned())
    }
}

impl From<&String> for TextBody {
    fn from(text: &String) -> TextBody {
        TextBody::Text(text.clone())
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::error::Result;

    use super::*;

    fn complete() -> Result<MailBuilder> {
        Ok(Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .text("Hi"))
    }

    #[test]
    fn collects_every_field() -> Result<()> {
        let mail = Mail::builder()
            .from(Mailbox::named("Ada", "ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .to(vec![Mailbox::named("Grace Hopper", "grace@example.com")?])
            .cc([Mailbox::new("carol@example.com")?])
            .bcc([Mailbox::new("dan@example.com")?])
            .reply_to([Mailbox::new("replies@example.com")?])
            .subject("Hello")
            .html(ViewHandle::empty())
            .text("Hello there")
            .attachments([Attachment::new("invoice.pdf", "application/pdf", b"%PDF-")])
            .headers([(
                "List-Unsubscribe".to_owned(),
                "<mailto:stop@example.com>".to_owned(),
            )])
            .in_reply_to("<earlier@example.com>")
            .references("<earlier@example.com>")
            .date(SystemTime::UNIX_EPOCH)
            .message_id("<mail@example.com>")
            .build()?;

        assert_eq!(mail.from(), &Mailbox::named("Ada", "ada@example.com")?);
        assert_eq!(
            mail.to(),
            [
                Mailbox::new("bob@example.com")?,
                Mailbox::named("Grace Hopper", "grace@example.com")?,
            ]
        );
        assert_eq!(mail.cc(), [Mailbox::new("carol@example.com")?]);
        assert_eq!(mail.bcc(), [Mailbox::new("dan@example.com")?]);
        assert_eq!(mail.reply_to(), [Mailbox::new("replies@example.com")?]);
        assert_eq!(mail.subject(), "Hello");
        assert!(mail.html().is_some());
        assert_eq!(mail.text(), &TextBody::Text("Hello there".to_owned()));
        assert_eq!(mail.attachments().len(), 1);
        assert_eq!(
            mail.headers(),
            [(
                "List-Unsubscribe".to_owned(),
                "<mailto:stop@example.com>".to_owned()
            )]
        );
        assert_eq!(mail.in_reply_to(), Some("<earlier@example.com>"));
        assert_eq!(mail.references(), Some("<earlier@example.com>"));
        assert_eq!(mail.date(), Some(SystemTime::UNIX_EPOCH));
        assert_eq!(mail.message_id(), Some("<mail@example.com>"));

        Ok(())
    }

    #[test]
    fn optional_fields_default_to_empty() -> Result<()> {
        let mail = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .html(ViewHandle::empty())
            .build()?;

        assert_eq!(mail.cc(), []);
        assert_eq!(mail.bcc(), []);
        assert_eq!(mail.reply_to(), []);
        assert_eq!(mail.subject(), "");
        assert_eq!(mail.text(), &TextBody::FromHtml);
        assert_eq!(mail.attachments(), []);
        assert_eq!(mail.headers(), []);
        assert_eq!(mail.date(), None);
        assert_eq!(mail.message_id(), None);

        Ok(())
    }

    #[test]
    fn recipient_setters_append_across_calls() -> Result<()> {
        let mail = complete()?
            .to([
                Mailbox::new("carol@example.com")?,
                Mailbox::new("dan@example.com")?,
            ])
            .build()?;

        assert_eq!(mail.to().len(), 3);

        Ok(())
    }

    #[test]
    fn any_recipient_kind_completes_the_mail() -> Result<()> {
        let only_bcc = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .bcc([Mailbox::new("dan@example.com")?])
            .text("Hi")
            .build();
        assert!(only_bcc.is_ok());

        let only_cc = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .cc([Mailbox::new("carol@example.com")?])
            .text("Hi")
            .build();
        assert!(only_cc.is_ok());

        Ok(())
    }

    #[test]
    fn incomplete_mail_is_rejected() -> Result<()> {
        let missing_from = Mail::builder()
            .to([Mailbox::new("bob@example.com")?])
            .text("Hi")
            .build();
        assert!(matches!(missing_from, Err(MailError::MissingFrom)));

        let missing_recipients = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .text("Hi")
            .build();
        assert!(matches!(
            missing_recipients,
            Err(MailError::MissingRecipients)
        ));

        let missing_body = complete()?.text(TextBody::FromHtml).build();
        assert!(matches!(missing_body, Err(MailError::MissingBody)));

        let opted_out_without_html = complete()?.text(TextBody::None).build();
        assert!(matches!(
            opted_out_without_html,
            Err(MailError::MissingBody)
        ));

        let orphan_inline = complete()?
            .attachments([Attachment::inline("logo", "image/png", b"\x89PNG")])
            .build();
        assert!(matches!(orphan_inline, Err(MailError::InlineWithoutHtml)));

        Ok(())
    }

    #[test]
    fn invalid_declarations_are_rejected() -> Result<()> {
        let content_type = complete()?
            .attachments([Attachment::new("a.bin", "not a mime type", b"")])
            .build();
        assert!(matches!(
            content_type,
            Err(MailError::InvalidContentType { .. })
        ));

        let header_name = complete()?
            .headers([("Bad Name".to_owned(), "value".to_owned())])
            .build();
        assert!(matches!(
            header_name,
            Err(MailError::InvalidHeaderName { .. })
        ));

        Ok(())
    }
}
