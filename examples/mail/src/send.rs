use topcoat::{
    Result,
    context::Cx,
    mail::{Attachment, Mail, Mailbox, send},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
    view::{ViewExt, view},
};

use crate::{FERRIS, GETTING_STARTED, Recipient};

#[route(POST)]
pub(crate) async fn send_welcome(cx: &Cx, Form(recipient): Form<Recipient>) -> Result<SeeOther> {
    let mail = Mail::builder()
        .from(Mailbox::named("Topcoat", "welcome@example.com")?)
        .to([Mailbox::named(&recipient.name, &recipient.address)?])
        .reply_to([Mailbox::new("support@example.com")?])
        .subject(format!("Welcome, {}!", recipient.name))
        // Mail clients support less CSS than browsers, so the styles stay
        // simple and inline.
        .html(
            view! {
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
            }
            .single()
            .await?,
        )
        // Without a `text` call, the plain-text alternative is derived from
        // the HTML.
        .attachments([
            Attachment::inline("ferris", "image/png", FERRIS),
            Attachment::new("getting-started.txt", "text/plain", GETTING_STARTED),
        ])
        .headers([(
            "List-Unsubscribe".to_owned(),
            "<mailto:unsubscribe@example.com>".to_owned(),
        )])
        .build()?;

    send(cx, mail).await?;

    Ok(see_other(href!(crate::sent::sent).resolve(cx)))
}
