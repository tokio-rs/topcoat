Topcoat sends email through a [`Transport`] that you choose, such as an SMTP server or a folder of files. Declare a mail with the [`mail!`] macro, then send it from any handler with [`send`].

Everything below lives in `topcoat::mail` and needs the `mail` feature. The SMTP transport also needs the `mail-smtp` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["mail", "mail-smtp"] }
```

# Setup

Put the transport your application sends through into a [`MailConfig`], and register it with [`mail`](RouterBuilderMailExt::mail) on the router builder:

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

Handlers send through whatever transport is registered. You can use a file transport in development and SMTP in production without changing any code that sends mail.

# Declaring and sending mail

The [`mail!`] macro declares a [`Mail`] as a list of `name: value` fields: the addresses, the subject, an HTML body written like a [`view!`](crate::view::view) body, attachments, and custom headers. [`send`] then delivers it through the registered transport:

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

An address can be a string, a `(name, address)` pair, or a [`Mailbox`], and fields that take several addresses also accept collections. By default, a plain-text version of the mail is derived from the HTML body, because spam filters rate mail without one lower. See the [`mail!`] reference for all fields, and [`MailBuilder`] to build a mail without the macro.

[`send`] returns a [`Receipt`] with the `Message-ID` of the sent mail. Save it if you want to send a later mail in the same thread, using the `in_reply_to` and `references` fields. A receipt only means that the transport accepted the mail, not that the mail reached an inbox. Sending fails with a [`SendError`] when the mail is incomplete (no `From` address, no recipients, or no body) or when delivery fails.

# Attachments

The `attachments` field adds files to the mail. A regular [`Attachment`] is shown to the recipient as a file to download. An [inline attachment](Attachment::inline) is shown inside the HTML body, where a `cid:` URL references its content id:

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

Topcoat includes three transports. Each implements the [`Transport`] trait, and you can write your own.

## SMTP

[`SmtpTransport`] (behind the `mail-smtp` feature) sends mail to an SMTP server, such as the submission server of a mail provider or your own mail server. It keeps a pool of connections and reuses them across sends. Create one for a host with [`relay`](SmtpTransport::relay) (TLS on port 465) or [`starttls`](SmtpTransport::starttls) (STARTTLS on port 587), or from a connection URL with [`from_url`](SmtpTransport::from_url), which fits well into a single environment variable:

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

[`FileTransport`] writes each mail to an `.eml` file instead of delivering it, so you can develop without a mail server and open the mail in any mail client. File names start with the time the mail was sent, so a directory listing shows mail in the order it was sent. The file also keeps the `Bcc` header, so you can see every recipient.

## Memory in tests

[`MemoryTransport`] keeps sent mail in memory, so tests can check it. All clones of a `MemoryTransport` share the same list of sent mail, so a test can keep one clone and give another to the code under test:

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

It builds the full message just like a delivering transport, so a mail that would fail to send also fails in the test.

## Custom transports

Implement [`Transport`] to deliver mail some other way, such as through the HTTP API of a mail provider. For APIs that accept raw messages, [`Mail::formatted`] renders the mail into the standard RFC 5322 message format.
