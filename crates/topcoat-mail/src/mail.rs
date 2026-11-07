//! Mail messages and the builder that assembles them.

use std::time::SystemTime;

use lettre::message::{
    Message,
    header::{ContentType, HeaderName},
};
use topcoat_core::context::Cx;
use topcoat_view::{View, ViewExt};
use uuid::Uuid;

use crate::{Attachment, Mailbox, text::PlainText};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    from: Mailbox,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    reply_to: Vec<Mailbox>,
    subject: String,
    body: Body,
    attachments: Vec<Attachment>,
    in_reply_to: Option<String>,
    references: Option<String>,
    headers: Vec<(String, String)>,
    date: SystemTime,
    message_id: String,
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

    /// The rendered HTML body, if any.
    #[must_use]
    pub fn html(&self) -> Option<&str> {
        match &self.body {
            Body::Html { html, .. } => Some(html),
            Body::Text(_) => None,
        }
    }

    /// The plain-text body, if any.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match &self.body {
            Body::Html { text, .. } => text.as_deref(),
            Body::Text(text) => Some(text),
        }
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

    /// The `Date` header value.
    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    /// The `Message-ID` header value.
    #[must_use]
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    /// The mail in its RFC 5322 wire form: the headers followed by the MIME
    /// body. The output omits `Bcc` recipients.
    ///
    /// Use this for services that accept raw messages or tests that inspect
    /// message bytes. You do not need to call it before sending.
    #[must_use]
    pub fn formatted(&self) -> Vec<u8> {
        Message::from(self).formatted()
    }

    /// The HTML and plain-text bodies.
    pub(crate) fn body(&self) -> &Body {
        &self.body
    }
}

/// The bodies of a [`Mail`]: HTML with an optional plain-text alternative,
/// or plain text alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    /// Rendered HTML, with the plain-text alternative if there is one.
    Html { html: String, text: Option<String> },
    /// Plain text alone.
    Text(String),
}

/// Assembles a [`Mail`], created by [`Mail::builder`].
#[derive(Clone, Debug, Default)]
pub struct MailBuilder {
    from: Option<Mailbox>,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    reply_to: Vec<Mailbox>,
    subject: String,
    html: Option<String>,
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
    pub fn from(mut self, from: Mailbox) -> Self {
        self.from = Some(from);
        self
    }

    /// Adds `To` recipients.
    #[must_use]
    pub fn to(mut self, to: impl IntoIterator<Item = Mailbox>) -> Self {
        self.to.extend(to);
        self
    }

    /// Adds `Cc` recipients.
    #[must_use]
    pub fn cc(mut self, cc: impl IntoIterator<Item = Mailbox>) -> Self {
        self.cc.extend(cc);
        self
    }

    /// Adds `Bcc` recipients, who receive the mail without appearing in
    /// its headers.
    #[must_use]
    pub fn bcc(mut self, bcc: impl IntoIterator<Item = Mailbox>) -> Self {
        self.bcc.extend(bcc);
        self
    }

    /// Adds `Reply-To` addresses, where replies are directed instead of the
    /// `From` address.
    #[must_use]
    pub fn reply_to(mut self, reply_to: impl IntoIterator<Item = Mailbox>) -> Self {
        self.reply_to.extend(reply_to);
        self
    }

    /// Sets the subject line.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Sets the HTML body, resolving the view and rendering it with `cx`.
    ///
    /// # Errors
    ///
    /// Returns the error the view fails with.
    ///
    /// # Panics
    ///
    /// Panics if the view is live.
    pub async fn html(mut self, cx: &Cx, view: impl View) -> topcoat_core::error::Result<Self> {
        self.html = Some(view.single().await?.render(cx));
        Ok(self)
    }

    /// Sets the plain-text body, the fallback for clients that do not
    /// render HTML.
    ///
    /// Accepts the text itself or a [`TextBody`] variant. Without a call,
    /// the text is derived from the HTML body; pass [`TextBody::None`] to
    /// send the HTML alone.
    #[must_use]
    pub fn text(mut self, text: impl Into<TextBody>) -> Self {
        self.text = text.into();
        self
    }

    /// Adds attachments.
    #[must_use]
    pub fn attachments(mut self, attachments: impl IntoIterator<Item = Attachment>) -> Self {
        self.attachments.extend(attachments);
        self
    }

    /// Adds custom `(name, value)` headers, such as `List-Unsubscribe`.
    #[must_use]
    pub fn headers<N, V>(mut self, headers: impl IntoIterator<Item = (N, V)>) -> Self
    where
        N: Into<String>,
        V: Into<String>,
    {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(name, value)| (name.into(), value.into())),
        );
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

    /// Sets the `Date` header. Without an explicit date, the time of
    /// building is used.
    #[must_use]
    pub fn date(mut self, date: SystemTime) -> Self {
        self.date = Some(date);
        self
    }

    /// Sets the `Message-ID`. Without an explicit id, a unique one at the
    /// sender's domain is generated.
    #[must_use]
    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Finishes the builder into a [`Mail`], deriving the plain-text body
    /// from the HTML unless it was declared.
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

        let body = match (self.html, self.text) {
            (Some(html), TextBody::FromHtml) => {
                let text = PlainText::from_html(&html);
                Body::Html {
                    html,
                    text: (!text.is_empty()).then_some(text),
                }
            }
            (Some(html), TextBody::None) => Body::Html { html, text: None },
            (Some(html), TextBody::Text(text)) => Body::Html {
                html,
                text: Some(text),
            },
            (None, TextBody::Text(text)) => Body::Text(text),
            (None, TextBody::FromHtml | TextBody::None) => return Err(MailError::MissingBody),
        };
        if matches!(body, Body::Text(_))
            && self
                .attachments
                .iter()
                .any(|attachment| attachment.content_id().is_some())
        {
            return Err(MailError::InlineWithoutHtml);
        }

        let message_id = self
            .message_id
            .unwrap_or_else(|| format!("<{}@{}>", Uuid::new_v4().simple(), from.domain()));

        Ok(Mail {
            from,
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            reply_to: self.reply_to,
            subject: self.subject,
            body,
            attachments: self.attachments,
            in_reply_to: self.in_reply_to,
            references: self.references,
            headers: self.headers,
            date: self.date.unwrap_or_else(SystemTime::now),
            message_id,
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
    /// Derive the text from the HTML body when the mail is built.
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
    use topcoat::view::view;
    use topcoat_core::error::Result;

    use super::*;

    fn addressed() -> Result<MailBuilder> {
        Ok(Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?]))
    }

    fn complete() -> Result<MailBuilder> {
        Ok(addressed()?.text("Hi"))
    }

    #[tokio::test]
    async fn collects_every_field() -> Result<()> {
        let cx = &Cx::new();
        let mail = Mail::builder()
            .from(Mailbox::named("Ada", "ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .to(vec![Mailbox::named("Grace Hopper", "grace@example.com")?])
            .cc([Mailbox::new("carol@example.com")?])
            .bcc([Mailbox::new("dan@example.com")?])
            .reply_to([Mailbox::new("replies@example.com")?])
            .subject("Hello")
            .html(cx, view! { <p>"Hello"</p> })
            .await?
            .text("Hello there")
            .attachments([Attachment::new("invoice.pdf", "application/pdf", b"%PDF-")])
            .headers([("List-Unsubscribe", "<mailto:stop@example.com>")])
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
        assert_eq!(mail.html(), Some("<p>Hello</p>"));
        assert_eq!(mail.text(), Some("Hello there"));
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
        assert_eq!(mail.date(), SystemTime::UNIX_EPOCH);
        assert_eq!(mail.message_id(), "<mail@example.com>");

        Ok(())
    }

    #[test]
    fn optional_fields_default_to_empty() -> Result<()> {
        let mail = complete()?.build()?;

        assert_eq!(mail.cc(), []);
        assert_eq!(mail.bcc(), []);
        assert_eq!(mail.reply_to(), []);
        assert_eq!(mail.subject(), "");
        assert_eq!(mail.html(), None);
        assert_eq!(mail.attachments(), []);
        assert_eq!(mail.headers(), []);
        assert_eq!(mail.in_reply_to(), None);
        assert_eq!(mail.references(), None);

        Ok(())
    }

    #[tokio::test]
    async fn text_is_derived_from_the_html_unless_declared() -> Result<()> {
        let cx = &Cx::new();

        let derived = addressed()?
            .html(
                cx,
                view! {
                    <p>
                        "Hi "
                        <b>"Bob"</b>
                    </p>
                },
            )
            .await?
            .build()?;
        assert_eq!(derived.text(), Some("Hi Bob"));

        let declared = addressed()?
            .html(cx, view! { <p>"Hi"</p> })
            .await?
            .text("Hey")
            .build()?;
        assert_eq!(declared.text(), Some("Hey"));

        let opted_out = addressed()?
            .html(cx, view! { <p>"Hi"</p> })
            .await?
            .text(TextBody::None)
            .build()?;
        assert_eq!(opted_out.text(), None);

        let nothing_to_derive = addressed()?
            .html(cx, view! { <img src="cid:logo"> })
            .await?
            .build()?;
        assert_eq!(nothing_to_derive.text(), None);

        Ok(())
    }

    #[test]
    fn message_id_and_date_are_generated() -> Result<()> {
        let before = SystemTime::now();
        let first = complete()?.build()?;
        let second = complete()?.build()?;

        assert!(first.message_id().starts_with('<'));
        assert!(first.message_id().ends_with("@example.com>"));
        assert_ne!(first.message_id(), second.message_id());
        assert!(first.date() >= before);

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
        let blind_copy = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .bcc([Mailbox::new("dan@example.com")?])
            .text("Hi")
            .build();
        assert!(blind_copy.is_ok());

        let carbon_copy = Mail::builder()
            .from(Mailbox::new("ada@example.com")?)
            .cc([Mailbox::new("carol@example.com")?])
            .text("Hi")
            .build();
        assert!(carbon_copy.is_ok());

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

        let missing_body = addressed()?.build();
        assert!(matches!(missing_body, Err(MailError::MissingBody)));

        let opted_out_without_html = addressed()?.text(TextBody::None).build();
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

        let header_name = complete()?.headers([("Bad Name", "value")]).build();
        assert!(matches!(
            header_name,
            Err(MailError::InvalidHeaderName { .. })
        ));

        Ok(())
    }
}
