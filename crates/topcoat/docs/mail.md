Declare email with [`mail!`] and deliver it with [`send`]. A [`Transport`] determines how messages are delivered.

Everything below is re-exported from `topcoat::mail` and gated behind the `mail` feature. The SMTP transport additionally needs the `mail-smtp` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["mail", "mail-smtp"] }
```

# Setup

Choose a transport in [`MailConfig`] and register it with the router's [`mail`](RouterBuilderMailExt::mail) method:

```rust
use topcoat::{
    mail::{FileTransport, MailConfig, RouterBuilderMailExt},
    router::{Router, RouterBuilderDiscoverExt},
};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .mail(
            MailConfig::builder()
                .transport(FileTransport::new("target/mail"))
                .build(),
        )
        .build()
}
```

Handlers use the registered transport. You can change delivery settings without changing the code that sends messages.

# Declaring and sending mail

The [`mail!`] macro creates a [`Mail`] from `name: value` fields. Its `html` field accepts [`view!`](crate::view::view) markup:

```rust
use topcoat::{
    Result,
    context::Cx,
    mail::{mail, send},
    router::route,
};

#[route(POST "/api/welcome")]
async fn welcome(cx: &Cx) -> Result<&'static str> {
    let mail = mail! {
        from: ("Topcoat", "welcome@example.com"),
        to: "ada@example.com",
        subject: "Welcome, Ada!",
        html: {
            <h1>"Welcome!"</h1>
            <p>"Your account is ready."</p>
        },
    }?;

    send(cx, mail).await?;

    Ok("sent")
}
```

Addresses accept strings, `(name, address)` pairs, or [`Mailbox`] values. Recipient fields also accept collections. Topcoat derives a plain-text body from the HTML by default. See [`mail!`] for field details or use [`MailBuilder`] to create a message without the macro.

[`send`] returns a [`Receipt`] with the message's `Message-ID`. Use that ID in `in_reply_to` and `references` to link a later reply. The receipt confirms that the transport accepted the message. It does not confirm inbox delivery.

Sending returns a [`SendError`] if the message lacks a sender, recipients, or body, or if delivery fails.

# Attachments

Add files with the `attachments` field. Use [`Attachment::new`] for a downloadable file or [`Attachment::inline`] for content referenced by a `cid:` URL in the HTML:

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::mail::{Attachment, mail};
# async fn example(cx: &Cx) -> Result<()> {
let mail = mail! {
    subject: "Your invoice",
    html: {
        cx =>
        <img src="cid:logo" alt="Example logo">
        <p>"The invoice is attached."</p>
    },
    attachments: [
        Attachment::inline("logo", "image/png", b"\x89PNG"),
        Attachment::new("invoice.pdf", "application/pdf", b"%PDF-"),
    ],
}?;
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

[`MemoryTransport`] captures messages for tests. Clones share the captured messages, so the test can inspect what the application sent:

```rust
use topcoat::{
    context::CxTestBuilder,
    mail::{MailConfig, MemoryTransport, Mailbox, mail, send},
};

# async fn test() -> topcoat::Result<()> {
let transport = MemoryTransport::new();
let config = MailConfig::builder().transport(transport.clone()).build();
let cx = CxTestBuilder::new().app_context(config).build();

send(&cx, mail! { from: "ada@example.com", to: "bob@example.com", text: "Hi" }?).await?;

assert_eq!(transport.sent().len(), 1);
assert_eq!(transport.sent()[0].to(), [Mailbox::new("bob@example.com")?]);
# Ok(())
# }
```

The memory transport validates message content before recording it. It does not check whether an external service would accept the message.

## Custom transports

Implement [`Transport`] to deliver through anything else, such as a mail provider's HTTP API. [`Mail::formatted`] renders the RFC 5322 wire form for APIs that accept raw messages.
