use topcoat_core_grammar::testing::assert_deterministic;
use topcoat_mail_grammar::mail::Mail;

#[test]
fn mail() {
    assert_deterministic(|| syn::parse_str::<Mail>(""));
    assert_deterministic(|| {
        syn::parse_str::<Mail>(
            r#"
            from: Mailbox::named("Ada", "ada@example.com")?,
            to: [Mailbox::new("bob@example.com")?, Mailbox::new("grace@example.com")?],
            cc: Mailbox::new("carol@example.com")?,
            bcc: [Mailbox::new("dan@example.com")?],
            reply_to: Mailbox::new("replies@example.com")?,
            subject: "Analytical engines",
            html: { cx => <p>"The engine weaves algebraic patterns."</p> },
            text: "The engine weaves algebraic patterns.",
            attachments: [Attachment::new("invoice.pdf", "application/pdf", b"%PDF-")],
            headers: [("List-Unsubscribe", "<mailto:stop@example.com>")],
            in_reply_to: "<earlier@example.com>",
            references: "<earlier@example.com>",
            date: SystemTime::UNIX_EPOCH,
            message_id: "<mail@example.com>",
            "#,
        )
    });
}
