Build email with [`Mail::builder`] and deliver it with [`send`]. A [`Transport`] determines how messages are delivered.

Everything below is re-exported from `topcoat::mail` and gated behind the `mail` feature. The SMTP transport additionally needs the `mail-smtp` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.12.2", features = ["mail", "mail-smtp"] }
```

# Setup

Choose a transport, wrap it in a [`Mailer`], and add it to the router's context:

```rust,standalone_crate
use topcoat::{
    mail::{FileTransport, Mailer},
    router::{Router, RouterBuilderDiscoverExt, module_router},
};

pub fn router() -> Router {
    module_router!()
        .discover()
        .with(Mailer::new(FileTransport::new("target/mail")))
        .build()
}
```

Handlers use the registered transport. You can change delivery settings without changing the code that sends messages.

# Declaring and sending mail

[`Mail::builder`] creates a [`Mail`]. [`html`](MailBuilder::html) renders a view, such as [`view!`](crate::view::view) markup, into the HTML body. Mail cannot stream, so the view must not be live:

```rust
// src/app/api/welcome.rs
use topcoat::{
    Result,
    context::Cx,
    mail::{Mail, Mailbox, send},
    router::route,
    view::view,
};

#[route(POST)]
async fn welcome(cx: &Cx) -> Result<&'static str> {
    let mail = Mail::builder()
        .from(Mailbox::named("Topcoat", "welcome@example.com")?)
        .to([Mailbox::new("ada@example.com")?])
        .subject("Welcome, Ada!")
        .html(
            cx,
            view! {
                <h1>"Welcome!"</h1>
                <p>"Your account is ready."</p>
            },
        )
        .await?
        .build()?;

    send(cx, mail).await?;

    Ok("sent")
}
```

Create addresses with [`Mailbox::new`] or [`Mailbox::named`], or parse the `Ada Lovelace <ada@example.com>` form. Recipient methods take a collection of addresses and add to the recipients on each call. Topcoat derives a plain-text body from the HTML by default. Set [`text`](MailBuilder::text) to supply your own wording, or pass [`TextBody::None`] to send only HTML.

[`send`] returns a [`Receipt`] with the message's `Message-ID`. Use that ID in `in_reply_to` and `references` to link a later reply. The receipt confirms that the transport accepted the message. It does not confirm inbox delivery.

[`build`](MailBuilder::build) returns a [`MailError`] if the message lacks a sender, recipients, or body, or if an attachment or header is invalid. Sending returns a [`SendError`] if delivery fails.

# Attachments

Add files with [`attachments`](MailBuilder::attachments). Use [`Attachment::new`] for a downloadable file or [`Attachment::inline`] for content referenced by a `cid:` URL in the HTML:

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::{mail::{Attachment, Mail, Mailbox}, view::view};
# async fn example(cx: &Cx) -> Result<()> {
let mail = Mail::builder()
#   .from(Mailbox::new("billing@example.com")?)
#   .to([Mailbox::new("ada@example.com")?])
    .subject("Your invoice")
    .html(
        cx,
        view! {
            <img src="cid:logo" alt="Example logo">
            <p>"The invoice is attached."</p>
        },
    )
    .await?
    .attachments([
        Attachment::inline("logo", "image/png", b"\x89PNG"),
        Attachment::new("invoice.pdf", "application/pdf", b"%PDF-"),
    ])
    .build()?;
# Ok(())
# }
```

# Transports

Choose a transport for your delivery environment, or implement [`Transport`] for your own service.

## SMTP

[`SmtpTransport`] submits messages to an SMTP server and reuses connections across sends. Enable `mail-smtp` to use it. [`relay`](SmtpTransport::relay) uses implicit TLS on port 465, and [`starttls`](SmtpTransport::starttls) uses STARTTLS on port 587. You can also configure the connection with a URL:

```rust,no_run
# #[cfg(feature = "mail-smtp")]
# fn example() -> Result<(), topcoat::mail::SmtpError> {
use topcoat::mail::SmtpTransport;

let explicit = SmtpTransport::relay("smtp.example.com")?
    .credentials("username", "password")
    .build();

let from_url = SmtpTransport::from_url("smtps://user:pass@smtp.example.com:465")?.build();
# Ok(())
# }
```

## Files during development

[`FileTransport`] writes each message to an `.eml` file for local inspection. Filenames include the send time.

## Memory in tests

Use [`MemoryTransport`] in tests to collect outgoing messages. Add a `Mailer` to an empty `Cx`, then call `send` with that context. Keep a clone of the transport to inspect the messages:

```rust
use topcoat::{
    context::Cx,
    mail::{Mail, Mailbox, Mailer, MemoryTransport, send},
};

# async fn test() -> topcoat::Result<()> {
let transport = MemoryTransport::new();
let cx = Cx::new().with(Mailer::new(transport.clone()));

let mail = Mail::builder()
    .from(Mailbox::new("ada@example.com")?)
    .to([Mailbox::new("bob@example.com")?])
    .text("Hi")
    .build()?;

send(&cx, mail).await?;

assert_eq!(transport.sent().len(), 1);
assert_eq!(transport.sent()[0].to(), [Mailbox::new("bob@example.com")?]);
# Ok(())
# }
```

A passing test does not mean a real mail service will accept the message.

## Custom transports

Implement [`Transport`] to deliver through anything else, such as a mail provider's HTTP API. [`Mail::formatted`] renders the RFC 5322 wire form for APIs that accept raw messages.
