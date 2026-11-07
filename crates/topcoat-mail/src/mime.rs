//! Conversion of a [`Mail`] into lettre's MIME message.

use lettre::message::{
    Attachment as MimeAttachment, Message, MultiPart, SinglePart,
    header::{ContentType, HeaderName, HeaderValue},
};

use crate::{Attachment, Mail, attachment::Disposition, mail::Body};

/// Assembles the message with its envelope. The envelope reaches every
/// recipient, while the headers omit `Bcc`.
impl From<&Mail> for Message {
    fn from(mail: &Mail) -> Message {
        let mut builder = Message::builder()
            .from(mail.from().into())
            .date(mail.date())
            .message_id(Some(mail.message_id().to_owned()));
        for to in mail.to() {
            builder = builder.to(to.into());
        }
        for cc in mail.cc() {
            builder = builder.cc(cc.into());
        }
        for bcc in mail.bcc() {
            builder = builder.bcc(bcc.into());
        }
        for reply_to in mail.reply_to() {
            builder = builder.reply_to(reply_to.into());
        }
        if !mail.subject().is_empty() {
            builder = builder.subject(mail.subject());
        }
        if let Some(in_reply_to) = mail.in_reply_to() {
            builder = builder.in_reply_to(in_reply_to.to_owned());
        }
        if let Some(references) = mail.references() {
            builder = builder.references(references.to_owned());
        }
        for (name, value) in mail.headers() {
            let name = HeaderName::new_from_ascii(name.clone())
                .expect("header names are checked by `MailBuilder::build`");
            builder = builder.raw_header(HeaderValue::new(name, value.clone()));
        }

        let message = match Part::from(mail) {
            Part::Single(part) => builder.singlepart(part),
            Part::Multi(part) => builder.multipart(part),
        };
        message.expect("sender and recipients are checked by `MailBuilder::build`")
    }
}

impl From<&Attachment> for SinglePart {
    fn from(attachment: &Attachment) -> SinglePart {
        let content_type = ContentType::parse(attachment.content_type())
            .expect("content types are checked by `MailBuilder::build`");
        let part = match attachment.disposition() {
            Disposition::Attached { filename } => MimeAttachment::new(filename.clone()),
            Disposition::Inline { content_id } => MimeAttachment::new_inline(content_id.clone()),
        };
        part.body(attachment.content().to_vec(), content_type)
    }
}

/// A node of the MIME body tree. lettre types single parts and multiparts
/// differently, so the tree is built from whichever each step produces.
enum Part {
    Single(SinglePart),
    Multi(MultiPart),
}

impl Part {
    /// Adds this part as the next child of `parent`.
    fn append_to(self, parent: MultiPart) -> MultiPart {
        match self {
            Part::Single(part) => parent.singlepart(part),
            Part::Multi(part) => parent.multipart(part),
        }
    }
}

/// Arranges the bodies and attachments: the HTML body is `related` to its
/// inline attachments, the text body is its `alternative`, and downloadable
/// attachments are `mixed` around both.
impl From<&Mail> for Part {
    fn from(mail: &Mail) -> Part {
        let (inline, attached): (Vec<&Attachment>, Vec<&Attachment>) = mail
            .attachments()
            .iter()
            .partition(|attachment| attachment.content_id().is_some());

        let content = match mail.body() {
            Body::Text(text) => Part::Single(SinglePart::plain(text.clone())),
            Body::Html { html, text } => {
                let mut html = Part::Single(SinglePart::html(html.clone()));
                if !inline.is_empty() {
                    let mut related = html.append_to(MultiPart::related().build());
                    for attachment in inline {
                        related = related.singlepart(attachment.into());
                    }
                    html = Part::Multi(related);
                }
                match text {
                    None => html,
                    // RFC 2046: alternatives ascend in order of preference.
                    Some(text) => Part::Multi(html.append_to(
                        MultiPart::alternative().singlepart(SinglePart::plain(text.clone())),
                    )),
                }
            }
        };

        if attached.is_empty() {
            return content;
        }
        let mut mixed = content.append_to(MultiPart::mixed().build());
        for attachment in attached {
            mixed = mixed.singlepart(attachment.into());
        }
        Part::Multi(mixed)
    }
}

#[cfg(test)]
mod tests {
    use topcoat::view::view;
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{MailBuilder, Mailbox, TextBody};

    fn base() -> MailBuilder {
        Mail::builder()
            .from(Mailbox::named("Ada", "ada@example.com").unwrap())
            .to([Mailbox::new("bob@example.com").unwrap()])
    }

    fn formatted(mail: &Mail) -> String {
        String::from_utf8(mail.formatted()).unwrap()
    }

    #[test]
    fn text_only_is_a_single_plain_part() {
        let mail = base().subject("Hello").text("Hi Bob").build().unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("From: Ada <ada@example.com>\r\n"));
        assert!(wire.contains("To: bob@example.com\r\n"));
        assert!(wire.contains("Subject: Hello\r\n"));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(wire.contains("Hi Bob"));
        assert!(!wire.contains("multipart"));
    }

    #[tokio::test]
    async fn html_with_derived_text_is_an_alternative() {
        let cx = &Cx::new();
        let mail = base()
            .html(
                cx,
                view! {
                    <h1>"Hi"</h1>
                    <p>"Bye now"</p>
                },
            )
            .await
            .unwrap()
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/alternative;"));
        assert!(wire.contains("Content-Type: text/plain; charset=utf-8\r\n"));
        assert!(wire.contains("Bye now"));
    }

    #[tokio::test]
    async fn html_alone_is_a_single_html_part() {
        let cx = &Cx::new();
        let mail = base()
            .html(cx, view! { <h1>"Hi"</h1> })
            .await
            .unwrap()
            .text(TextBody::None)
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: text/html; charset=utf-8\r\n"));
        assert!(wire.contains("<h1>Hi</h1>"));
        assert!(!wire.contains("multipart"));
    }

    #[tokio::test]
    async fn both_bodies_are_alternatives_with_html_preferred() {
        let cx = &Cx::new();
        let mail = base()
            .text("Hi Bob")
            .html(cx, view! { <h1>"Hi"</h1> })
            .await
            .unwrap()
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/alternative;"));
        let plain = wire.find("Content-Type: text/plain").unwrap();
        let html = wire.find("Content-Type: text/html").unwrap();
        // RFC 2046: alternatives ascend in order of preference.
        assert!(plain < html);
    }

    #[tokio::test]
    async fn inline_attachments_relate_to_the_html_body() {
        let cx = &Cx::new();
        let mail = base()
            .text("Hi Bob")
            .html(cx, view! { <img src="cid:logo"> })
            .await
            .unwrap()
            .attachments([Attachment::inline("logo", "image/png", b"\x89PNG")])
            .build()
            .unwrap();
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
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("Content-Type: multipart/mixed;"));
        assert!(wire.contains("Content-Disposition: attachment; filename=\"invoice.pdf\"\r\n"));
    }

    #[test]
    fn bcc_recipients_reach_the_envelope_but_not_the_headers() {
        let mail = base()
            .bcc([Mailbox::new("dan@example.com").unwrap()])
            .text("Hi")
            .build()
            .unwrap();

        let message = Message::from(&mail);
        let recipients: Vec<String> = message
            .envelope()
            .to()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(recipients.contains(&"dan@example.com".to_owned()));

        let wire = String::from_utf8(message.formatted()).unwrap();
        assert!(!wire.contains("Bcc"));
        assert!(!wire.contains("dan@example.com"));
    }

    #[test]
    fn message_id_and_date_are_written() {
        let mail = base()
            .text("Hi")
            .message_id("<mail@example.com>")
            .date(std::time::SystemTime::UNIX_EPOCH)
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("Message-ID: <mail@example.com>\r\n"));
        assert!(wire.contains("Date: Thu, 01 Jan 1970 00:00:00 +0000\r\n"));
    }

    #[test]
    fn custom_headers_are_written() {
        let mail = base()
            .text("Hi")
            .headers([("List-Unsubscribe", "<mailto:stop@example.com>")])
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("List-Unsubscribe: <mailto:stop@example.com>\r\n"));
    }

    #[test]
    fn threading_headers_are_written() {
        let mail = base()
            .text("Hi")
            .in_reply_to("<earlier@example.com>")
            .references("<earlier@example.com>")
            .build()
            .unwrap();
        let wire = formatted(&mail);

        assert!(wire.contains("In-Reply-To: <earlier@example.com>\r\n"));
        assert!(wire.contains("References: <earlier@example.com>\r\n"));
    }
}
