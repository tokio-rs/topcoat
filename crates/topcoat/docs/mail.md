Topcoat sends email through a pluggable [`Transport`]. Declare a mail with the [`mail!`] macro, then deliver it from any handler with [`send`].

Everything below is re-exported from `topcoat::mail` and gated behind the `mail` feature. The SMTP transport additionally needs the `mail-smtp` feature.

# Setup

Choose a transport in [`MailConfig`] and register it with [`RouterBuilderMailExt::mail`]:

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

Handlers send through whichever transport is registered, so swapping it (a file transport in development, SMTP in production) changes nothing at the call sites.

# Declaring and sending mail

Build a message with [`mail!`], then call [`send`] to pass it to the configured transport:

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

A plain-text alternative is generated from HTML by default. See [`mail!`] for address formats and message fields, or use [`MailBuilder`] to build a message without the macro.

[`send`] returns a [`Receipt`] carrying the sent mail's `Message-ID`. Store it to thread a later mail onto this one through the `in_reply_to` and `references` fields. A receipt means the delivery mechanism accepted the mail, not that it reached an inbox. Sending fails with a [`SendError`] when the mail is incomplete (no `From` address, no recipients, or no body) or the delivery itself fails.

# Attachments

The `attachments` field carries files with the mail. A downloadable [`Attachment`] is presented to the recipient as a file; an [inline attachment](Attachment::inline) is displayed where the HTML body references its content id through a `cid:` URL:

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

A [`Transport`] determines where mail is sent.

## SMTP

[`SmtpTransport`] (behind the `mail-smtp` feature) submits to an SMTP server: a mail provider's submission endpoint or your own mail server. Connections are pooled and reused across sends. Point it at a host with [`relay`](SmtpTransport::relay) (implicit TLS on port 465) or [`starttls`](SmtpTransport::starttls) (STARTTLS on port 587), or configure it from a connection URL, the form that fits a single environment variable:

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

[`FileTransport`] writes each message as an `.eml` file for inspection during development. It does not deliver mail.

## Memory in tests

[`MemoryTransport`] captures sent mail in memory for tests to assert on. Every clone shares the same capture, so a test keeps one clone and hands the other to the code under test:

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

It assembles the mail exactly as a delivering transport would, so a mail that would fail to send fails in the test too.

## Custom transports

Implement [`Transport`] to deliver through anything else, such as a mail provider's HTTP API. [`Mail::formatted`] renders the RFC 5322 wire form for APIs that accept raw messages.
