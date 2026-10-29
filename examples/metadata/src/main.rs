mod products;

use topcoat::{
    Result,
    metadata::{Metadata, Url},
    router::{Slot, href, layout, module_router, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            // Canonical URLs are absolute, so the router needs to know where
            // the application is reachable.
            .base_url("http://localhost:3000")
            .build(),
    )
    .await
    .unwrap();
}

// Every page starts with this metadata. A page only sets the fields it wants
// to change.
#[layout(with(
    Metadata::new()
        .title("Acme")
        .description("Acme sells everything you need for tea.")
        .canonical(Url::Current)
))]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::metadata::tags()
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href=(href!(home))>"Home"</a>
                    " "
                    <a href=(href!(products::products))>"Products"</a>
                    " "
                    <a href=(href!(about::page))>"About"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}

// Keeps all of the layout's metadata.
#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Acme"</h1>
        <p>"View the page source to see the metadata of each page."</p>
    })
}

mod about {
    use topcoat::{
        Result,
        context::Cx,
        metadata::{Metadata, metadata},
        router::page,
        view::{View, view},
    };

    // Sets its own title and removes the layout's description.
    #[page(with(Metadata::new().title("About").no_description()))]
    pub async fn page(cx: &Cx) -> Result<impl View> {
        // Code can read the metadata of the current page too.
        let metadata = metadata(cx).await?;

        Ok(view! {
            if let Some(title) = metadata.title() {
                <h1>(title)</h1>
            }
            <p>"This page has no description."</p>
        })
    }
}
