use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    mail::{Attachment, FileTransport, MailConfig, RouterBuilderMailExt, mail, send},
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, see_other},
        layout, page, route,
    },
    view::view,
};

// The file transport writes every mail as an `.eml` file instead of delivering
// it, so the example needs no mail server. A real application would register
// an `SmtpTransport` here instead.
//
// The outbox path is relative to this example's Cargo manifest.
const OUTBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/outbox");

#[tokio::main]
async fn main() {
    // Configure the mail system to write messages to the local outbox.
    let config = MailConfig::builder()
        .transport(FileTransport::new(OUTBOX))
        .build();

    // Discover the routes, register the mail configuration, and start the
    // server at http://127.0.0.1:3000 by default.
    topcoat::start(Router::builder().discover().mail(config).build())
        .await
        .unwrap();
}

#[layout("/")]
async fn root(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Mail"</title>
                topcoat::dev::script()
            </head>
            <body>(slot?)</body>
        </html>
    }
}

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"Send a welcome mail"</h1>

        // Submitting this form creates a local `.eml` file.
        <form method="POST" action="/send">
            <input name="name" placeholder="Name" required="true">
            <input type="email" name="address" placeholder="Address" required="true">
            <button>"send"</button>
        </form>

        <p>"Nothing leaves the machine: the mail is written to a file."</p>
        <a href="/sent">"Outbox"</a>
    }
}

#[derive(Deserialize)]
struct Recipient {
    name: String,
    address: String,
}

// The image is embedded inline in the email, while the notes are attached
// as a regular text file.
const FERRIS: &[u8] = include_bytes!("./ferris.png");

const GETTING_STARTED: &str = "\
1. Start the dev server with `cargo run`.
2. Open http://localhost:3000.
3. Read the guides at https://docs.rs/topcoat.
";

#[route(POST "/send")]
async fn send_welcome(cx: &Cx, Form(recipient): Form<Recipient>) -> Result<SeeOther> {
    // Build a multipart email containing HTML, a generated text alternative,
    // an inline image, a file attachment, and a custom header.
    let mail = mail! {
        from: ("Topcoat", "welcome@example.com"),

        // Recipient fields accept a mailbox, address string, or
        // `(name, address)` pair.
        to: (&recipient.name, &recipient.address),

        reply_to: "support@example.com",
        subject: format!("Welcome, {}!", recipient.name),

        // Email clients support less CSS than browsers, so the styles
        // are kept simple and inline.
        html: {
            <div style="font-family: sans-serif; max-width: 30rem">
                // `cid:ferris` references the inline attachment below.
                <img src="cid:ferris" alt="Ferris the crab" width="120">

                <h1 style="font-size: 1.25rem">
                    "Welcome, "
                    (&recipient.name)
                    "!"
                </h1>

                <p>"Your account is ready. The attached notes get you started."</p>
            </div>
        },

        // No explicit `text` field is provided, so Topcoat derives the
        // plain-text alternative from the HTML content.
        attachments: [
            Attachment::inline("ferris", "image/png", FERRIS),
            Attachment::new("getting-started.txt", "text/plain", GETTING_STARTED),
        ],

        headers: ("List-Unsubscribe", "<mailto:unsubscribe@example.com>"),
    }?;

    // Send through the configured transport. In this example, sending means
    // writing the message to the local outbox.
    send(cx, mail).await?;

    Ok(see_other("/sent"))
}

#[page("/sent")]
async fn sent() -> Result {
    let files = outbox()?;

    view! {
        <h1>"Outbox"</h1>

        <p>
            "The mail was written to "
            <code>(OUTBOX)</code>
            ". Open one of these files in a mail client to read it as the \
             recipient would."
        </p>

        <ul>
            for file in files {
                <li>(file)</li>
            }
        </ul>

        <a href="/">"Send another"</a>
    }
}

// Return the generated messages ordered from oldest to newest.
fn outbox() -> Result<Vec<String>> {
    // The directory does not exist until the first message is sent.
    let Ok(entries) = std::fs::read_dir(OUTBOX) else {
        return Ok(Vec::new());
    };

    let mut files = entries
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<String>>>()?;

    files.sort();

    Ok(files)
}
