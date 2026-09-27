mod send;
mod sent;

use serde::Deserialize;
use topcoat::{Result, mail::{FileTransport, MailConfig, mail, send}, router::{Slot, href, layout, module_router, page}, view::{View, view}};

const OUTBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/outbox");

#[tokio::main]
async fn main() {
    // The file transport writes every mail as an `.eml` file instead of
    // delivering it, so the example needs no mail server. A real application
    // would use an `SmtpTransport` here.
    let config = MailConfig::builder()
        .transport(FileTransport::new(OUTBOX))
        .build();

    topcoat::start(module_router!().mail(config).build())
        .await
        .unwrap();
}

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Mail"</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Send a welcome mail"</h1>

        <form method="POST" action=(href!(crate::send::send_welcome))>
            <input name="name" placeholder="Name" required="true">
            <input type="email" name="address" placeholder="Address" required="true">
            <button>"send"</button>
        </form>

        <p>"Nothing leaves the machine: the mail is written to a file."</p>
        <a href=(href!(crate::sent::sent))>"Outbox"</a>
    })
}

#[derive(Deserialize)]
struct Recipient {
    name: String,
    address: String,
}

const FERRIS: &[u8] = include_bytes!("./ferris.png");

const GETTING_STARTED: &str = "\
1. Start the dev server with `cargo run`.
2. Open http://localhost:3000.
3. Read the guides at https://docs.rs/topcoat.
";

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
