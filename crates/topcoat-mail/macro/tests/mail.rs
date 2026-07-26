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
        to: [
            Mailbox::new("bob@example.com")?,
            Mailbox::new("grace@example.com")?,
        ],
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
        to: [
            Mailbox::new("bob@example.com")?,
            Mailbox::new("grace@example.com")?,
        ],
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
        html: {
            cx =>
            <p>
                "Hello, "
                (name)
                "!"
            </p>
        },
    }?;

    assert_eq!(
        mail.html().map(|html| html.render(cx)),
        Some("<p>Hello, Ada!</p>".to_owned())
    );

    Ok(())
}

#[tokio::test]
async fn mailboxes_accept_strings_and_pairs() -> Result<()> {
    let mail = mail! {
        from: "Ada Lovelace <ada@example.com>",
        to: [
            ("Bob", "bob@example.com"),
            ("Grace Hopper", "grace@example.com"),
        ],
        cc: ("Carol", "carol@example.com"),
        bcc: "dan@example.com",
    }?;

    assert_eq!(
        mail.from(),
        Some(&"Ada Lovelace <ada@example.com>".parse()?)
    );
    assert_eq!(
        mail.to(),
        [
            Mailbox::named("Bob", "bob@example.com")?,
            Mailbox::named("Grace Hopper", "grace@example.com")?,
        ]
    );
    assert_eq!(mail.cc(), [Mailbox::named("Carol", "carol@example.com")?]);
    assert_eq!(mail.bcc(), [Mailbox::new("dan@example.com")?]);

    Ok(())
}

#[tokio::test]
async fn mailboxes_accept_vecs_and_slices() -> Result<()> {
    let recipients = vec![
        Mailbox::new("bob@example.com")?,
        Mailbox::new("grace@example.com")?,
    ];
    let copies = ["carol@example.com", "dan@example.com"];

    let mail = mail! { to: recipients.clone(), cc: copies, bcc: &recipients[..1] }?;

    assert_eq!(mail.to(), recipients);
    assert_eq!(
        mail.cc(),
        [
            Mailbox::new("carol@example.com")?,
            Mailbox::new("dan@example.com")?,
        ]
    );
    assert_eq!(mail.bcc(), &recipients[..1]);

    Ok(())
}

#[tokio::test]
async fn attachments_and_headers_accept_collections() -> Result<()> {
    let attachments = vec![
        Attachment::new("invoice.pdf", "application/pdf", b"%PDF-"),
        Attachment::new("logo.png", "image/png", b"PNG"),
    ];

    let mail = mail! {
        attachments: attachments.clone(),
        headers: vec![("X-One", "1"), ("X-Two", "2")],
    }?;

    assert_eq!(mail.attachments(), attachments);
    assert_eq!(
        mail.headers(),
        [
            ("X-One".to_owned(), "1".to_owned()),
            ("X-Two".to_owned(), "2".to_owned()),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn invalid_mailbox_strings_fail_the_mail() {
    assert!(mail! { to: "not an address" }.is_err());
}

#[tokio::test]
async fn field_values_can_use_the_question_mark_operator() {
    async fn build() -> Result<Mail> {
        mail! { to: Mailbox::new("not an address")? }
    }

    assert!(build().await.is_err());
}
