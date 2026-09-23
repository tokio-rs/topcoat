Declares a [`Mail`] as a list of `name: value` fields.

Each field calls the [`MailBuilder`] method with the same name, and converts the value where needed. Addresses can be written as strings or `(name, address)` pairs, and the `html` field takes an inline [`view!`] body. The macro evaluates to a `Result<Mail>`, so a value that fails to convert, such as an invalid address string, becomes an error you handle with `?`.

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

A mail only declares its content. The MIME structure, the encodings, and the list of recipients for delivery are built when you pass the mail to [`send`], which delivers it through the [`Transport`] of the app's [`MailConfig`].

# Fields

Fields can appear in any order, but each at most once. Declaring a field twice is a compile error.

- `from`: the sender, a single address.
- `to`, `cc`, `bcc`, `reply_to`: the recipients, a single address or a collection.
- `subject`: the subject line.
- `html`: the HTML body, a braced view body or a [`ViewHandle`] expression.
- `text`: the plain-text body, derived from the HTML body by default.
- `attachments`: files sent with the mail, a single [`Attachment`] or a collection.
- `headers`: custom headers, a single `(name, value)` pair or a collection.
- `in_reply_to`, `references`: the threading headers for replies.
- `date`, `message_id`: generated when the mail is sent, unless declared.

# Addresses

`from` takes a single address. The recipient fields take a single address or a collection. Each address can be a [`Mailbox`], a string in the bare (`ada@example.com`) or display-name (`Ada Lovelace <ada@example.com>`) form, or a `(name, address)` pair. A collection can mix these forms. See [`TryIntoMailboxes`] for all accepted values.

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

A braced `html` value is a [`view!`] body. Mail clients support much less CSS than browsers, so keep mail markup simple and put styles inline. When the body renders components or anything else that needs the request context, start it with `cx =>`, like a [`view!`] call in a plain function:

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

A braced body must not contain live content, such as a [`live!`] region. The mail is rendered once, so it cannot stream updates.

An `html` value without braces is an expression of type [`ViewHandle`]. To use a view you built elsewhere, resolve it with [`ViewExt::single`] first:

```rust
# use topcoat::{Result, context::Cx};
# use topcoat::mail::{Mail, mail};
use topcoat::view::{ViewExt, view};

async fn notice(cx: &Cx) -> Result<Mail> {
    let body = view! { cx => <p>"Scheduled maintenance tonight."</p> };
    mail! {
        to: "ada@example.com",
        html: body.single().await?,
    }
}
```

# The Plain-Text Body

Spam filters rate mail without a plain-text version lower, so by default the text body is derived from the HTML body when the mail is sent. Declare `text` to write your own text, or pass [`TextBody::None`] to send only the HTML:

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

`attachments` takes a single [`Attachment`] or a collection. A regular attachment is shown to the recipient as a file to download. An [inline attachment](struct.Attachment.html#method.inline) is shown inside the HTML body, where a `cid:` URL references its content id. `headers` adds custom `(name, value)` headers to the mail:

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

The macro expands to an awaited async block, so it can only be used inside an async function. In return, field values can use `.await` and `?`, and their errors become the error of the macro:

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
[`ViewExt::single`]: ../view/trait.ViewExt.html#method.single
[`ViewHandle`]: ../view/struct.ViewHandle.html
[`live!`]: ../view/macro.live.html
[`send`]: fn.send.html
[`view!`]: ../view/macro.view.html
