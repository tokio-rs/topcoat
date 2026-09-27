Declares a [`Mail`] as a list of `name: value` fields.

Fields correspond to [`MailBuilder`] methods. The macro also accepts convenient forms such as address strings and inline [`view!`] markup. It returns `Result<Mail>`, so invalid field values return an error:

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

The macro creates the message without sending it. Call [`send`] to deliver it through the [`Transport`] registered in [`MailConfig`].

# Fields

Fields may appear in any order. Repeating a field is a compile error.

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

`from` takes one address. Recipient fields take one address or a collection. Use a [`Mailbox`], a string such as `ada@example.com` or `Ada Lovelace <ada@example.com>`, or a `(name, address)` pair. See [`TryIntoMailboxes`] for supported collections.

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

# The HTML Body

A braced `html` value uses [`view!`] syntax. Add a leading `cx =>` when the view needs the request context:

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

Pass an unbraced [`View`] expression to use an existing view.

# The Plain-Text Body

By default, Topcoat derives plain text from the HTML body. Set `text` to supply your own wording, or use [`TextBody::None`] to send only HTML:

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

# Attachments And Headers

`attachments` accepts one [`Attachment`] or a collection. Use an [inline attachment](struct.Attachment.html#method.inline) for content referenced by a `cid:` URL in the HTML. The `headers` field accepts custom `(name, value)` pairs:

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

# Fallible And Async Values

Use the macro in an async context. Field expressions can use `.await` and `?`, and their errors become the macro's result:

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
