use topcoat::{
    Result,
    router::{href, page},
    view::{View, view},
};

use crate::{OUTBOX, home, outbox};

#[page]
pub(crate) async fn sent() -> Result<impl View> {
    let files = outbox()?;

    Ok(view! {
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

        <a href=(href!(home))>"Send another"</a>
    })
}
