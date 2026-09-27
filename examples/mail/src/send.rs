use topcoat::{
    Result,
    context::Cx,
    mail::{Attachment, mail, send},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::{FERRIS, GETTING_STARTED, Recipient};

#[route(POST)]
pub(crate) async fn send_welcome(cx: &Cx, Form(recipient): Form<Recipient>) -> Result<SeeOther> {
    let mail = mail! {
        from: ("Topcoat", "welcome@example.com"),

        // Recipient fields accept a mailbox, an address, or a
        // `(name, address)` pair.
        to: (&recipient.name, &recipient.address),

        reply_to: "support@example.com",
        subject: format!("Welcome, {}!", recipient.name),

        // Mail clients support less CSS than browsers, so the styles stay
        // simple and inline.
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

        // Without a `text` field, the plain-text alternative is derived from
        // the HTML.
        attachments: [
            Attachment::inline("ferris", "image/png", FERRIS),
            Attachment::new("getting-started.txt", "text/plain", GETTING_STARTED),
        ],

        headers: ("List-Unsubscribe", "<mailto:unsubscribe@example.com>"),
    }?;

    send(cx, mail).await?;

    Ok(see_other(href!(crate::sent::sent).resolve(cx)))
}
