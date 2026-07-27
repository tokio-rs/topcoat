Declares a [`Mail`] as a list of `name: value` fields.

Each field lowers to the [`MailBuilder`] method of the same name, with conversions layered on top: addresses can be written as strings or `(name, address)` pairs, and the `html` field takes an inline [`view!`] body. The macro is an expression producing `Result<Mail>`, so a field value that fails, such as an invalid address string, surfaces as an error at the invocation.

```rust
# use topcoat::Result;
# use topcoat::mail::mail;
# async fn example() -> Result<()> {
let mail = mail! {
    from: ("Topcoat", "welcome@example.com"),
    to: "ada@example.com",
    subject: "Welcome, Ada!",
    html: {
        <h1>"Welcome!"</h1>
        <p>"Your account is ready."</p>
    },
}?;
# Ok(())
# }
```

A mail declares its content only. The MIME structure, encodings, and the envelope are assembled when the mail is passed to [`send`], which delivers it through the [`Transport`] registered in the app's [`MailConfig`].

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

# The HTML Body

A braced `html` value is a [`view!`] body. Mail clients understand far less CSS than browsers do, so mail markup stays plain and carries its styles inline. When the body renders components or other markup that needs the request context, name it with a leading `cx =>`, just as in a plain-function `view!` call:

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

# The Plain-Text Body

Mail without a plain-text alternative scores worse with spam filters, so by default the text body is derived from the HTML body when the mail is assembled. Declare `text` to send your own wording instead, or pass [`TextBody::None`] to send the HTML alone:

```rust
# use topcoat::Result;
# use topcoat::mail::{TextBody, mail};
# async fn example() -> Result<()> {
let derived = mail! { html: { <p>"Hi"</p> } }?;
let declared = mail! { text: "Hi there" }?;
let html_alone = mail! { html: { <p>"Hi"</p> }, text: TextBody::None }?;

assert_eq!(derived.text(), &TextBody::FromHtml);
assert_eq!(declared.text(), &TextBody::Text("Hi there".to_owned()));
assert_eq!(html_alone.text(), &TextBody::None);
# Ok(())
# }
```

# Attachments And Headers

`attachments` takes a single [`Attachment`] or a collection. A downloadable attachment is presented to the recipient as a file; an [inline attachment](struct.Attachment.html#method.inline) is displayed where the HTML body references its content id through a `cid:` URL. `headers` adds custom `(name, value)` pairs to the message:

```rust
# use topcoat::Result;
# use topcoat::mail::{Attachment, mail};
# async fn example() -> Result<()> {
let mail = mail! {
    subject: "Your invoice",
    html: {
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

The macro expands to an awaited async block, so it must be used inside an async function. In exchange, field values can use `.await` and `?` directly, and their errors surface as the macro's own `Err`:

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
[`View`]: ../view/struct.View.html
[`send`]: fn.send.html
[`view!`]: ../view/macro.view.html
