//! Assembly of a [`Mail`] into its MIME wire form.

use lettre::message::{
    Attachment as MimeAttachment, Message, MessageBuilder, MultiPart, SinglePart,
    header::{ContentType, HeaderName, HeaderValue},
};
use topcoat_core::context::Cx;

use crate::{Attachment, Mail, SendError, TextBody, text::text_from_html};

impl Mail {
    /// Renders the mail into its RFC 5322 wire form.
    ///
    /// The bytes are what a transport puts on the wire: the header section
    /// followed by the MIME body tree, with `multipart/alternative` for a
    /// mail with both bodies, `multipart/related` around an HTML body with
    /// inline attachments, and `multipart/mixed` around downloadable
    /// attachments. The HTML body is rendered with `cx`, the plain-text
    /// alternative is derived from it unless the mail declares its
    /// [`TextBody`] otherwise, and a `Date` and `Message-ID` are generated
    /// for a mail that does not declare them, so two calls produce two
    /// distinct messages. `Bcc` recipients are omitted, as they are on the
    /// wire; they only ever appear in the envelope a transport derives from
    /// the mail itself.
    ///
    /// Transports assemble the mail themselves, so there is no need to call
    /// this before sending. It is for handing the wire form elsewhere: a
    /// mail provider's API that accepts raw messages, or a test asserting
    /// on the exact bytes.
    ///
    /// # Errors
    ///
    /// Returns [`SendError`] if the mail is incomplete (no `From` address,
    /// no recipients, or no body) or declares an attachment content type or
    /// custom header name that is invalid.
    pub fn formatted(&self, cx: &Cx) -> Result<Vec<u8>, SendError> {
        Ok(message(cx, self, BccHeader::Dropped)?.formatted())
    }
}

/// Whether the assembled message retains its `Bcc` header.
///
/// The wire form of a sent mail must not reveal `Bcc` recipients, so
/// assembly drops the header after deriving the envelope. [`Dropped`]
/// (`BccHeader::Dropped`) is therefore correct for every delivering
/// transport; [`Kept`](BccHeader::Kept) is for inspection output like
/// [`FileTransport`](crate::FileTransport)'s `.eml` files, where losing the
/// `Bcc` recipients would hide part of the mail.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BccHeader {
    /// Remove the `Bcc` header from the assembled message.
    Dropped,
    /// Keep the `Bcc` header in the assembled message.
    Kept,
}

/// Assembles the mail into a MIME message, rendering its HTML body with
/// `cx`.
///
/// The message carries its envelope (all recipients, including `Bcc`) and
/// a `Message-ID` and `Date`, generated unless the mail declared them.
pub(crate) fn message(cx: &Cx, mail: &Mail, bcc: BccHeader) -> Result<Message, SendError> {
    let builder = headers(mail, bcc)?;
    let content = body(cx, mail)?;

    let message = match content {
        Content::Single(part) => builder.singlepart(part),
        Content::Multi(part) => builder.multipart(part),
    };
    message.map_err(SendError::assembly)
}

/// The `Message-ID` of an assembled message, for the [`Receipt`].
///
/// [`Receipt`]: crate::Receipt
pub(crate) fn message_id(message: &Message) -> String {
    message
        .headers()
        .get_raw("Message-ID")
        .unwrap_or_default()
        .to_owned()
}

/// Builds the header section, validating the address requirements.
fn headers(mail: &Mail, bcc: BccHeader) -> Result<MessageBuilder, SendError> {
    let from = mail.from().ok_or(SendError::MissingFrom)?;
    if mail.to().is_empty() && mail.cc().is_empty() && mail.bcc().is_empty() {
        return Err(SendError::MissingRecipients);
    }

    let mut builder = Message::builder().from(from.to_lettre());
    for to in mail.to() {
        builder = builder.to(to.to_lettre());
    }
    for cc in mail.cc() {
        builder = builder.cc(cc.to_lettre());
    }
    for bcc in mail.bcc() {
        builder = builder.bcc(bcc.to_lettre());
    }
    for reply_to in mail.reply_to() {
        builder = builder.reply_to(reply_to.to_lettre());
    }
    if let BccHeader::Kept = bcc {
        builder = builder.keep_bcc();
    }

    if !mail.subject().is_empty() {
        builder = builder.subject(mail.subject());
    }
    if let Some(date) = mail.date() {
        builder = builder.date(date);
    }
    builder = builder.message_id(mail.message_id().map(ToOwned::to_owned));
    if let Some(in_reply_to) = mail.in_reply_to() {
        builder = builder.in_reply_to(in_reply_to.to_owned());
    }
    if let Some(references) = mail.references() {
        builder = builder.references(references.to_owned());
    }

    for (name, value) in mail.headers() {
        let header_name = HeaderName::new_from_ascii(name.clone())
            .map_err(|_| SendError::InvalidHeaderName { name: name.clone() })?;
        builder = builder.raw_header(HeaderValue::new(header_name, value.clone()));
    }

    Ok(builder)
}

/// A MIME body tree: lettre types single and multipart bodies differently,
/// so assembly carries whichever the mail produced.
enum Content {
    Single(SinglePart),
    Multi(MultiPart),
}

/// Builds the MIME body tree, rendering the HTML body with `cx`.
fn body(cx: &Cx, mail: &Mail) -> Result<Content, SendError> {
    let inline: Vec<&Attachment> = mail
        .attachments()
        .iter()
        .filter(|attachment| attachment.content_id().is_some())
        .collect();
    let attached: Vec<&Attachment> = mail
        .attachments()
        .iter()
        .filter(|attachment| attachment.filename().is_some())
        .collect();

    let html = mail.html().map(|view| view.render(cx));
    if html.is_none() && !inline.is_empty() {
        return Err(SendError::InlineWithoutHtml);
    }

    // The text body: declared, derived from the HTML, or absent.
    let text = match mail.text() {
        TextBody::Text(text) => Some(text.clone()),
        TextBody::None => None,
        TextBody::FromHtml => html
            .as_deref()
            .map(text_from_html)
            .filter(|text| !text.is_empty()),
    };

    // The HTML body, with its inline attachments alongside it.
    let html = html
        .map(|html| -> Result<Content, SendError> {
            let part = SinglePart::html(html);
            if inline.is_empty() {
                return Ok(Content::Single(part));
            }
            let mut related = MultiPart::related().singlepart(part);
            for attachment in inline {
                related = related.singlepart(attachment_part(attachment)?);
            }
            Ok(Content::Multi(related))
        })
        .transpose()?;

    // The text body as the fallback alternative to the HTML body.
    let content = match (text, html) {
        (None, None) => return Err(SendError::MissingBody),
        (None, Some(html)) => html,
        (Some(text), None) => Content::Single(SinglePart::plain(text)),
        (Some(text), Some(html)) => {
            let alternative = MultiPart::alternative().singlepart(SinglePart::plain(text));
            Content::Multi(match html {
                Content::Single(part) => alternative.singlepart(part),
                Content::Multi(part) => alternative.multipart(part),
            })
        }
    };

    // The downloadable attachments around whatever the bodies produced.
    if attached.is_empty() {
        return Ok(content);
    }
    let mut mixed = match content {
        Content::Single(part) => MultiPart::mixed().singlepart(part),
        Content::Multi(part) => MultiPart::mixed().multipart(part),
    };
    for attachment in attached {
        mixed = mixed.singlepart(attachment_part(attachment)?);
    }
    Ok(Content::Multi(mixed))
}

/// Converts an [`Attachment`] into its MIME part.
fn attachment_part(attachment: &Attachment) -> Result<SinglePart, SendError> {
    let content_type = ContentType::parse(attachment.content_type()).map_err(|_| {
        SendError::InvalidContentType {
            content_type: attachment.content_type().to_owned(),
        }
    })?;
    let part = match (attachment.filename(), attachment.content_id()) {
        (Some(filename), _) => MimeAttachment::new(filename.to_owned()),
        (None, Some(content_id)) => MimeAttachment::new_inline(content_id.to_owned()),
        (None, None) => unreachable!("an attachment is downloadable or inline"),
    };
    Ok(part.body(attachment.content().to_vec(), content_type))
}

#[cfg(test)]
mod tests {
    use topcoat_view::View;

    use super::*;
    use crate::Mailbox;

    fn base() -> crate::MailBuilder {
        Mail::builder()
            .from(Mailbox::named("Ada", "ada@example.com").unwrap())
            .to([Mailbox::new("bob@example.com").unwrap()])
    }

    fn formatted(mail: &Mail) -> String {
        String::from_utf8(mail.formatted(&Cx::default()).unwrap()).unwrap()
    }

    #[test]
    fn text_only_is_a_single_plain_part() {
        let mail = base().subject("Hello").text("Hi Bob").build();
        let wire = formatted(&mail);

        assert!(wire.contains("From: Ada <ada@example.com>\r\n"));
        assert!(wire.contains("To: bob@example.com\r\n"));
        assert!(wire.contains("Subject: Hello\r\n"));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(wire.contains("Hi Bob"));
        assert!(!wire.contains("multipart"));
    }

    #[test]
    fn html_derives_its_text_alternative_by_default() {
        let mail = base()
            .html(View::unescaped_unchecked("<h1>Hi</h1><p>Bye now</p>"))
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/alternative;"));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(wire.contains("Bye now"));
    }

    #[test]
    fn opted_out_text_leaves_a_single_html_part() {
        let mail = base()
            .html(View::unescaped_unchecked("<h1>Hi</h1>"))
            .text(TextBody::None)
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: text/html; charset=utf-8\r\n"));
        assert!(wire.contains("<h1>Hi</h1>"));
        assert!(!wire.contains("multipart"));
    }

    #[test]
    fn empty_derived_text_is_not_sent() {
        let mail = base()
            .html(View::unescaped_unchecked("<img src=\"x.png\">"))
            .build();
        let wire = formatted(&mail);

        assert!(!wire.contains("text/plain"));
        assert!(!wire.contains("multipart"));
    }

    #[test]
    fn both_bodies_are_alternatives_with_html_preferred() {
        let mail = base()
            .text("Hi Bob")
            .html(View::unescaped_unchecked("<h1>Hi</h1>"))
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/alternative;"));
        let plain = wire.find("Content-Type: text/plain").unwrap();
        let html = wire.find("Content-Type: text/html").unwrap();
        // RFC 2046: alternatives ascend in order of preference.
        assert!(plain < html);
    }

    #[test]
    fn inline_attachments_relate_to_the_html_body() {
        let mail = base()
            .text("Hi Bob")
            .html(View::unescaped_unchecked("<img src=\"cid:logo\">"))
            .attachments([Attachment::inline("logo", "image/png", b"\x89PNG")])
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/alternative;"));
        assert!(wire.contains("Content-Type: multipart/related;"));
        assert!(wire.contains("Content-ID: <logo>"));
        assert!(wire.contains("Content-Disposition: inline\r\n"));
    }

    #[test]
    fn downloadable_attachments_mix_with_the_bodies() {
        let mail = base()
            .text("Hi Bob")
            .attachments([Attachment::new("invoice.pdf", "application/pdf", b"%PDF-")])
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/mixed;"));
        assert!(wire.contains("Content-Disposition: attachment; filename=\"invoice.pdf\"\r\n"));
    }

    #[test]
    fn bcc_recipients_reach_the_envelope_but_not_the_wire() {
        let mail = base()
            .bcc([Mailbox::new("dan@example.com").unwrap()])
            .text("Hi")
            .build();

        let message = message(&Cx::default(), &mail, BccHeader::Dropped).unwrap();
        let recipients: Vec<String> = message
            .envelope()
            .to()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(recipients.contains(&"dan@example.com".to_owned()));

        let wire = formatted(&mail);
        assert!(!wire.contains("Bcc"));
        assert!(!wire.contains("dan@example.com"));
    }

    #[test]
    fn kept_bcc_stays_on_the_wire() {
        let mail = base()
            .bcc([Mailbox::new("dan@example.com").unwrap()])
            .text("Hi")
            .build();

        let message = message(&Cx::default(), &mail, BccHeader::Kept).unwrap();
        let wire = String::from_utf8(message.formatted()).unwrap();
        assert!(wire.contains("Bcc: dan@example.com\r\n"));
    }

    #[test]
    fn declared_message_id_and_date_are_respected() {
        let mail = base()
            .text("Hi")
            .message_id("<mail@example.com>")
            .date(std::time::SystemTime::UNIX_EPOCH)
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("Message-ID: <mail@example.com>\r\n"));
        assert!(wire.contains("Date: Thu, 01 Jan 1970 00:00:00 +0000\r\n"));
    }

    #[test]
    fn missing_message_id_and_date_are_generated() {
        let mail = base().text("Hi").build();

        let message = message(&Cx::default(), &mail, BccHeader::Dropped).unwrap();
        let id = message_id(&message);
        assert!(id.contains('@'), "generated Message-ID: {id:?}");
        assert!(message.headers().get_raw("Date").is_some());
    }

    #[test]
    fn custom_headers_are_written() {
        let mail = base()
            .text("Hi")
            .headers([(
                "List-Unsubscribe".to_owned(),
                "<mailto:stop@example.com>".to_owned(),
            )])
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("List-Unsubscribe: <mailto:stop@example.com>\r\n"));
    }

    #[test]
    fn threading_headers_are_written() {
        let mail = base()
            .text("Hi")
            .in_reply_to("<earlier@example.com>")
            .references("<earlier@example.com>")
            .build();
        let wire = formatted(&mail);

        assert!(wire.contains("In-Reply-To: <earlier@example.com>\r\n"));
        assert!(wire.contains("References: <earlier@example.com>\r\n"));
    }

    #[test]
    fn incomplete_mail_is_rejected() {
        let missing_from = Mail::builder()
            .to([Mailbox::new("bob@example.com").unwrap()])
            .text("Hi")
            .build();
        assert!(matches!(
            missing_from.formatted(&Cx::default()),
            Err(SendError::MissingFrom)
        ));

        let missing_recipients = Mail::builder()
            .from(Mailbox::new("ada@example.com").unwrap())
            .text("Hi")
            .build();
        assert!(matches!(
            missing_recipients.formatted(&Cx::default()),
            Err(SendError::MissingRecipients)
        ));

        let missing_body = base().build();
        assert!(matches!(
            missing_body.formatted(&Cx::default()),
            Err(SendError::MissingBody)
        ));

        let orphan_inline = base()
            .text("Hi")
            .attachments([Attachment::inline("logo", "image/png", b"\x89PNG")])
            .build();
        assert!(matches!(
            orphan_inline.formatted(&Cx::default()),
            Err(SendError::InlineWithoutHtml)
        ));
    }

    #[test]
    fn invalid_declarations_are_rejected() {
        let content_type = base()
            .text("Hi")
            .attachments([Attachment::new("a.bin", "not a mime type", b"")])
            .build();
        assert!(matches!(
            content_type.formatted(&Cx::default()),
            Err(SendError::InvalidContentType { .. })
        ));

        let header_name = base()
            .text("Hi")
            .headers([("Bad Name".to_owned(), "value".to_owned())])
            .build();
        assert!(matches!(
            header_name.formatted(&Cx::default()),
            Err(SendError::InvalidHeaderName { .. })
        ));
    }
}
