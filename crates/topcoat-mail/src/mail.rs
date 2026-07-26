//! Mail messages and the builder that assembles them.

use std::time::SystemTime;

use topcoat_view::View;

use crate::{Attachment, Mailbox};

/// A mail message: addresses, subject, bodies, and attachments.
///
/// A mail declares its content and leaves wire concerns -- the MIME
/// structure, encodings, and the envelope -- to the mailer that sends it.
/// Build one with [`Mail::builder`]:
///
/// ```
/// use topcoat_mail::{Mail, Mailbox};
///
/// let mail = Mail::builder()
///     .to([Mailbox::named("Ada Lovelace", "ada@example.com")?])
///     .subject("Analytical engines")
///     .text("The engine weaves algebraic patterns.")
///     .build();
/// # Ok::<(), topcoat_mail::AddressError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct Mail {
    from: Option<Mailbox>,
    to: Vec<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
    reply_to: Vec<Mailbox>,
    subject: String,
    html: Option<View>,
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

    /// The `From` address, if one was set.
    #[must_use]
    pub fn from(&self) -> Option<&Mailbox> {
        self.from.as_ref()
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
    pub fn html(&self) -> Option<&View> {
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
/// The `From` setter takes a single address as anything `Into<Mailbox>`.
/// The recipient setters (`to`, `cc`, `bcc`, `reply_to`) take any
/// collection convertible into a `Vec<Mailbox>` -- a `Vec`, an array, or a
/// slice -- and append on every call, as do `attachments` and `headers`.
/// Building never fails; addresses are validated when they are
/// constructed, and remaining wire concerns when the mail is sent.
///
/// The `mail!` macro builds on this and additionally converts address
/// strings, `(name, address)` pairs, and single values through
/// [`TryIntoMailboxes`](crate::TryIntoMailboxes) and its siblings.
#[derive(Clone, Debug, Default)]
pub struct MailBuilder {
    mail: Mail,
}

impl MailBuilder {
    /// Sets the `From` address.
    #[must_use]
    pub fn from(mut self, from: impl Into<Mailbox>) -> Self {
        self.mail.from = Some(from.into());
        self
    }

    /// Adds `To` recipients.
    #[must_use]
    pub fn to(mut self, to: impl Into<Vec<Mailbox>>) -> Self {
        self.mail.to.extend(to.into());
        self
    }

    /// Adds `Cc` recipients.
    #[must_use]
    pub fn cc(mut self, cc: impl Into<Vec<Mailbox>>) -> Self {
        self.mail.cc.extend(cc.into());
        self
    }

    /// Adds `Bcc` recipients, who receive the mail without appearing in
    /// its headers.
    #[must_use]
    pub fn bcc(mut self, bcc: impl Into<Vec<Mailbox>>) -> Self {
        self.mail.bcc.extend(bcc.into());
        self
    }

    /// Adds `Reply-To` addresses, where replies are directed instead of the
    /// `From` address.
    #[must_use]
    pub fn reply_to(mut self, reply_to: impl Into<Vec<Mailbox>>) -> Self {
        self.mail.reply_to.extend(reply_to.into());
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

    /// Sets the plain-text body, the fallback for clients that do not
    /// render HTML.
    ///
    /// Accepts the text itself or a [`TextBody`] variant. Without a call,
    /// the text is derived from the HTML body when the mail is assembled;
    /// pass [`TextBody::None`] to send the HTML alone.
    #[must_use]
    pub fn text(mut self, text: impl Into<TextBody>) -> Self {
        self.mail.text = text.into();
        self
    }

    /// Adds attachments.
    #[must_use]
    pub fn attachments(mut self, attachments: impl Into<Vec<Attachment>>) -> Self {
        self.mail.attachments.extend(attachments.into());
        self
    }

    /// Adds custom `(name, value)` headers, such as `List-Unsubscribe`.
    #[must_use]
    pub fn headers(mut self, headers: impl Into<Vec<(String, String)>>) -> Self {
        self.mail.headers.extend(headers.into());
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

/// One or more custom headers, converted from a single `(name, value)` pair
/// or a collection of pairs. The `mail!` macro's `headers` field accepts
/// anything implementing this trait.
pub trait IntoHeaders {
    /// Converts into `(name, value)` header pairs.
    fn into_headers(self) -> Vec<(String, String)>;
}

impl<N, V> IntoHeaders for (N, V)
where
    N: Into<String>,
    V: Into<String>,
{
    fn into_headers(self) -> Vec<(String, String)> {
        vec![(self.0.into(), self.1.into())]
    }
}

impl<N, V> IntoHeaders for Vec<(N, V)>
where
    N: Into<String>,
    V: Into<String>,
{
    fn into_headers(self) -> Vec<(String, String)> {
        self.into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect()
    }
}

impl<N, V, const M: usize> IntoHeaders for [(N, V); M]
where
    N: Into<String>,
    V: Into<String>,
{
    fn into_headers(self) -> Vec<(String, String)> {
        self.into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect()
    }
}

impl<N, V> IntoHeaders for &[(N, V)]
where
    N: Clone + Into<String>,
    V: Clone + Into<String>,
{
    fn into_headers(self) -> Vec<(String, String)> {
        self.iter()
            .map(|(name, value)| (name.clone().into(), value.clone().into()))
            .collect()
    }
}

/// The plain-text body of a mail: derived, declared, or absent.
///
/// Mail without a plain-text alternative scores worse with spam filters, so
/// the default, [`FromHtml`](TextBody::FromHtml), derives one from the
/// rendered HTML body when the mail is assembled. Declare the text yourself
/// or opt out through the builder's [`text`](MailBuilder::text) setter:
///
/// ```
/// use topcoat_mail::{Mail, TextBody};
///
/// let derived = Mail::builder().build();
/// let declared = Mail::builder().text("The engine weaves.").build();
/// let html_alone = Mail::builder().text(TextBody::None).build();
///
/// assert_eq!(derived.text(), &TextBody::FromHtml);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TextBody {
    /// Derive the text from the HTML body when the mail is assembled. A
    /// mail without an HTML body sends no text body.
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
    use super::*;
    use crate::AddressError;

    #[test]
    fn collects_every_field() -> Result<(), AddressError> {
        let mail = Mail::builder()
            .from(Mailbox::named("Ada", "ada@example.com")?)
            .to([Mailbox::new("bob@example.com")?])
            .to(vec![Mailbox::named("Grace Hopper", "grace@example.com")?])
            .cc([Mailbox::new("carol@example.com")?])
            .bcc([Mailbox::new("dan@example.com")?])
            .reply_to([Mailbox::new("replies@example.com")?])
            .subject("Hello")
            .html(View::empty())
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
            .build();

        assert_eq!(
            mail.from(),
            Some(&Mailbox::named("Ada", "ada@example.com")?)
        );
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
    fn defaults_are_empty() {
        let mail = Mail::builder().build();

        assert_eq!(mail.from(), None);
        assert!(mail.to().is_empty());
        assert!(mail.cc().is_empty());
        assert!(mail.bcc().is_empty());
        assert!(mail.reply_to().is_empty());
        assert_eq!(mail.subject(), "");
        assert!(mail.html().is_none());
        assert_eq!(mail.text(), &TextBody::FromHtml);
        assert!(mail.attachments().is_empty());
        assert!(mail.headers().is_empty());
        assert_eq!(mail.date(), None);
    }

    #[test]
    fn recipient_setters_append_across_calls() -> Result<(), AddressError> {
        let mail = Mail::builder()
            .to([Mailbox::new("bob@example.com")?])
            .to([
                Mailbox::new("carol@example.com")?,
                Mailbox::new("dan@example.com")?,
            ])
            .build();

        assert_eq!(mail.to().len(), 3);

        Ok(())
    }
}
