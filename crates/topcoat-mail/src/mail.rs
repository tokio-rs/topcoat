//! Mail messages and the builder that assembles them.

use std::time::SystemTime;

use topcoat_view::View;

use crate::{Attachment, MailAddress};

/// A mail message: addresses, subject, bodies, and attachments.
///
/// A mail declares its content and leaves wire concerns -- the MIME
/// structure, encodings, and the envelope -- to the mailer that sends it.
/// Build one with [`Mail::builder`]:
///
/// ```
/// use topcoat_mail::Mail;
///
/// let mail = Mail::builder()
///     .to(("Ada Lovelace", "ada@example.com"))
///     .subject("Analytical engines")
///     .text("The engine weaves algebraic patterns.")
///     .build();
/// ```
#[derive(Clone, Debug, Default)]
pub struct Mail {
    from: Option<MailAddress>,
    to: Vec<MailAddress>,
    cc: Vec<MailAddress>,
    bcc: Vec<MailAddress>,
    reply_to: Vec<MailAddress>,
    subject: String,
    html: Option<View>,
    text: Option<String>,
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

    /// The `From` address, if one was set.
    #[must_use]
    pub fn from(&self) -> Option<&MailAddress> {
        self.from.as_ref()
    }

    /// The `To` recipients.
    #[must_use]
    pub fn to(&self) -> &[MailAddress] {
        &self.to
    }

    /// The `Cc` recipients.
    #[must_use]
    pub fn cc(&self) -> &[MailAddress] {
        &self.cc
    }

    /// The `Bcc` recipients.
    #[must_use]
    pub fn bcc(&self) -> &[MailAddress] {
        &self.bcc
    }

    /// The `Reply-To` addresses.
    #[must_use]
    pub fn reply_to(&self) -> &[MailAddress] {
        &self.reply_to
    }

    /// The subject line.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The HTML body, if any.
    #[must_use]
    pub fn html(&self) -> Option<&View> {
        self.html.as_ref()
    }

    /// The plain-text body, if any.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
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
/// Address setters accept anything `Into<MailAddress>`, and the recipient
/// setters (`to`, `cc`, `bcc`, `reply_to`) append on every call. Building
/// never fails; addresses and content are validated when the mail is sent.
#[derive(Clone, Debug, Default)]
pub struct MailBuilder {
    mail: Mail,
}

impl MailBuilder {
    /// Sets the `From` address.
    #[must_use]
    pub fn from(mut self, from: impl Into<MailAddress>) -> Self {
        self.mail.from = Some(from.into());
        self
    }

    /// Adds a `To` recipient.
    #[must_use]
    pub fn to(mut self, to: impl Into<MailAddress>) -> Self {
        self.mail.to.push(to.into());
        self
    }

    /// Adds a `Cc` recipient.
    #[must_use]
    pub fn cc(mut self, cc: impl Into<MailAddress>) -> Self {
        self.mail.cc.push(cc.into());
        self
    }

    /// Adds a `Bcc` recipient, who receives the mail without appearing in
    /// its headers.
    #[must_use]
    pub fn bcc(mut self, bcc: impl Into<MailAddress>) -> Self {
        self.mail.bcc.push(bcc.into());
        self
    }

    /// Adds a `Reply-To` address, where replies are directed instead of the
    /// `From` address.
    #[must_use]
    pub fn reply_to(mut self, reply_to: impl Into<MailAddress>) -> Self {
        self.mail.reply_to.push(reply_to.into());
        self
    }

    /// Sets the subject line.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.mail.subject = subject.into();
        self
    }

    /// Sets the HTML body.
    #[must_use]
    pub fn html(mut self, html: View) -> Self {
        self.mail.html = Some(html);
        self
    }

    /// Sets the plain-text body.
    ///
    /// Set alongside `html`, it is the fallback for clients that do not
    /// render HTML.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.mail.text = Some(text.into());
        self
    }

    /// Adds an attachment.
    #[must_use]
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.mail.attachments.push(attachment);
        self
    }

    /// Adds a custom header, such as `List-Unsubscribe`.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.mail.headers.push((name.into(), value.into()));
        self
    }

    /// Marks the mail as a reply to the given message id.
    #[must_use]
    pub fn in_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.mail.in_reply_to = Some(message_id.into());
        self
    }

    /// Sets the `References` header linking the message ids of the thread.
    #[must_use]
    pub fn references(mut self, references: impl Into<String>) -> Self {
        self.mail.references = Some(references.into());
        self
    }

    /// Sets the `Date` header. Without an explicit date, the send time is
    /// used.
    #[must_use]
    pub fn date(mut self, date: SystemTime) -> Self {
        self.mail.date = Some(date);
        self
    }

    /// Sets the `Message-ID`. Without an explicit id, one is generated at
    /// send time.
    #[must_use]
    pub fn message_id(mut self, message_id: impl Into<String>) -> Self {
        self.mail.message_id = Some(message_id.into());
        self
    }

    /// Finishes the builder into a [`Mail`].
    #[must_use]
    pub fn build(self) -> Mail {
        self.mail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_every_field() {
        let mail = Mail::builder()
            .from(("Ada", "ada@example.com"))
            .to("bob@example.com")
            .to(("Grace Hopper", "grace@example.com"))
            .cc("carol@example.com")
            .bcc("dan@example.com")
            .reply_to("replies@example.com")
            .subject("Hello")
            .html(View::empty())
            .text("Hello there")
            .attachment(Attachment::new("invoice.pdf", "application/pdf", b"%PDF-"))
            .header("List-Unsubscribe", "<mailto:stop@example.com>")
            .in_reply_to("<earlier@example.com>")
            .references("<earlier@example.com>")
            .date(SystemTime::UNIX_EPOCH)
            .message_id("<mail@example.com>")
            .build();

        assert_eq!(
            mail.from(),
            Some(&MailAddress::named("Ada", "ada@example.com"))
        );
        assert_eq!(
            mail.to(),
            [
                MailAddress::new("bob@example.com"),
                MailAddress::named("Grace Hopper", "grace@example.com"),
            ]
        );
        assert_eq!(mail.cc(), [MailAddress::new("carol@example.com")]);
        assert_eq!(mail.bcc(), [MailAddress::new("dan@example.com")]);
        assert_eq!(mail.reply_to(), [MailAddress::new("replies@example.com")]);
        assert_eq!(mail.subject(), "Hello");
        assert!(mail.html().is_some());
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
        assert_eq!(mail.date(), Some(SystemTime::UNIX_EPOCH));
        assert_eq!(mail.message_id(), Some("<mail@example.com>"));
    }

    #[test]
    fn defaults_are_empty() {
        let mail = Mail::builder().build();

        assert_eq!(mail.from(), None);
        assert!(mail.to().is_empty());
        assert!(mail.cc().is_empty());
        assert!(mail.bcc().is_empty());
        assert!(mail.reply_to().is_empty());
        assert_eq!(mail.subject(), "");
        assert!(mail.html().is_none());
        assert!(mail.text().is_none());
        assert!(mail.attachments().is_empty());
        assert!(mail.headers().is_empty());
        assert_eq!(mail.date(), None);
    }
}
