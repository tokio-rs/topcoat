Declares a [`Mail`] as a list of `name: value` fields.

Use strings or `(name, address)` pairs for addresses, and a [`view!`] body for HTML. The macro returns `Result<Mail>` and reports invalid field values as errors.

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::mail::mail;
# async fn example(cx: &Cx) -> Result<()> {
let mail = mail! {
    from: ("Topcoat", "welcome@example.com"),
    to: "ada@example.com",
    subject: "Welcome, Ada!",
    html: {
        cx =>
        <h1>"Welcome!"</h1>
        <p>"Your account is ready."</p>
    },
}?;
# Ok(())
# }
```

Call [`send`] to deliver the message through the configured transport.

# Fields

Fields may appear in any order, each at most once; declaring a field twice is a compile error.

- `from`: the sender, a single address.
- `to`, `cc`, `bcc`, `reply_to`: the recipients, single addresses or collections.
- `subject`: the subject line.
- `html`: the HTML body, a braced view body or a [`View`] expression.
- `text`: the plain-text body, derived from the HTML body by default.
- `attachments`: files carried by the mail, a single [`Attachment`] or a collection.
- `headers`: custom headers, a single `(name, value)` pair or a collection.
- `in_reply_to`, `references`: the threading headers for replies.
- `date`, `message_id`: generated at send time unless declared.

# Addresses

`from` takes a single address; the recipient fields take one address or a collection. Every address position accepts a [`Mailbox`], an address string in bare (`ada@example.com`) or display-name (`Ada Lovelace <ada@example.com>`) form, or a `(name, address)` pair, and the flavors mix freely inside a collection; see [`TryIntoMailboxes`].

```rust
# use topcoat::Result;
# use topcoat::mail::{Mailbox, mail};
# async fn example() -> Result<()> {
let mail = mail! {
    from: "Grace Hopper <grace@example.com>",
    to: [("Ada", "ada@example.com"), ("Bob", "bob@example.com")],
    cc: Mailbox::new("carol@example.com")?,
    bcc: ["dan@example.com", "eve@example.com"],
}?;
# Ok(())
# }
```

# HTML body

A braced `html` value is a [`view!`] body. Add `cx =>` when the view needs a request context:

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::mail::{Mail, mail};
async fn welcome(cx: &Cx, name: &str) -> Result<Mail> {
    mail! {
        to: "ada@example.com",
        subject: format!("Welcome, {name}!"),
        html: {
            cx =>
            <p>
                "Hello, "
                (name)
                "!"
            </p>
        },
    }
}
```

An unbraced `html` value is an expression, so a prebuilt [`View`] can be passed as-is.

# Plain-text body

Plain text is derived from the HTML by default. Set `text` to supply your own wording, or use [`TextBody::None`] to omit the plain-text body:

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::mail::{TextBody, mail};
# async fn example(cx: &Cx) -> Result<()> {
let derived = mail! { html: { cx => <p>"Hi"</p> } }?;
let declared = mail! { text: "Hi there" }?;
let html_alone = mail! { html: { cx => <p>"Hi"</p> }, text: TextBody::None }?;

assert_eq!(derived.text(), &TextBody::FromHtml);
assert_eq!(declared.text(), &TextBody::Text("Hi there".to_owned()));
assert_eq!(html_alone.text(), &TextBody::None);
# Ok(())
# }
```

# Attachments and headers

`attachments` takes a single [`Attachment`] or a collection. A downloadable attachment is presented to the recipient as a file; an [inline attachment](struct.Attachment.html#method.inline) is displayed where the HTML body references its content id through a `cid:` URL. `headers` adds custom `(name, value)` pairs to the message:

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
    headers: ("List-Unsubscribe", "<mailto:stop@example.com>"),
}?;
# Ok(())
# }
```

# Fallible and async values

Use the macro inside an async function. Field values can use `.await` and `?`; their errors become the macro's `Err`:

```rust
# use topcoat::Result;
# use topcoat::mail::{Mail, Mailbox, mail};
# async fn subscribers() -> Result<Vec<Mailbox>> { Ok(Vec::new()) }
# async fn example() -> Result<Mail> {
mail! {
    from: "news@example.com",
    to: subscribers().await?,
    subject: "What changed this week",
}
# }
```

[`Attachment`]: struct.Attachment.html
[`Mail`]: struct.Mail.html
[`MailBuilder`]: struct.MailBuilder.html
[`MailConfig`]: struct.MailConfig.html
[`Mailbox`]: struct.Mailbox.html
[`TextBody`]: enum.TextBody.html
[`TextBody::None`]: enum.TextBody.html#variant.None
[`Transport`]: trait.Transport.html
[`TryIntoMailboxes`]: trait.TryIntoMailboxes.html
[`View`]: ../view/trait.View.html
[`send`]: fn.send.html
[`view!`]: ../view/macro.view.html
