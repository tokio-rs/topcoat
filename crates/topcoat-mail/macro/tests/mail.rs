use std::time::SystemTime;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::mail::{Attachment, Mail, Mailbox, TextBody, mail};

#[tokio::test]
async fn empty_body_builds_a_default_mail() -> Result<()> {
    let mail = mail! {}?;

    assert_eq!(mail.from(), None);
    assert!(mail.to().is_empty());
    assert_eq!(mail.subject(), "");
    assert!(mail.html().is_none());
    assert_eq!(mail.text(), &TextBody::FromHtml);

    Ok(())
}

#[tokio::test]
async fn collects_every_field() -> Result<()> {
    let mail = mail! {
        from: Mailbox::named("Ada", "ada@example.com")?,
        to: [Mailbox::new("bob@example.com")?, Mailbox::new("grace@example.com")?],
        cc: Mailbox::new("carol@example.com")?,
        bcc: [Mailbox::new("dan@example.com")?],
        reply_to: Mailbox::new("replies@example.com")?,
        subject: "Analytical engines",
        html: { <p>"The engine weaves algebraic patterns."</p> },
        text: "The engine weaves algebraic patterns.",
        attachments: [Attachment::new("invoice.pdf", "application/pdf", b"%PDF-")],
        headers: [("List-Unsubscribe", "<mailto:stop@example.com>")],
        in_reply_to: "<earlier@example.com>",
        references: "<earlier@example.com>",
        date: SystemTime::UNIX_EPOCH,
        message_id: "<mail@example.com>",
    }?;

    assert_eq!(
        mail.from(),
        Some(&Mailbox::named("Ada", "ada@example.com")?)
    );
    assert_eq!(mail.to().len(), 2);
    assert_eq!(mail.cc(), [Mailbox::new("carol@example.com")?]);
    assert_eq!(mail.bcc(), [Mailbox::new("dan@example.com")?]);
    assert_eq!(mail.reply_to(), [Mailbox::new("replies@example.com")?]);
    assert_eq!(mail.subject(), "Analytical engines");
    assert_eq!(
        mail.html().map(|html| html.render(&Cx::default())),
        Some("<p>The engine weaves algebraic patterns.</p>".to_owned())
    );
    assert_eq!(
        mail.text(),
        &TextBody::Text("The engine weaves algebraic patterns.".to_owned())
    );
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

#[tokio::test]
async fn additive_fields_append_in_written_order() -> Result<()> {
    let mail = mail! {
        to: [Mailbox::new("bob@example.com")?, Mailbox::new("grace@example.com")?],
    }?;

    assert_eq!(
        mail.to(),
        [
            Mailbox::new("bob@example.com")?,
            Mailbox::new("grace@example.com")?,
        ]
    );

    Ok(())
}

#[tokio::test]
async fn html_renders_dynamic_parts_against_the_named_context() -> Result<()> {
    let cx = &Cx::default();
    let name = "Ada";

    let mail = mail! {
        html: { cx => <p>"Hello, "(name)"!"</p> },
    }?;

    assert_eq!(
        mail.html().map(|html| html.render(cx)),
        Some("<p>Hello, Ada!</p>".to_owned())
    );

    Ok(())
}

#[tokio::test]
async fn field_values_can_use_the_question_mark_operator() {
    async fn build() -> Result<Mail> {
        mail! { to: Mailbox::new("not an address")? }
    }

    assert!(build().await.is_err());
}
