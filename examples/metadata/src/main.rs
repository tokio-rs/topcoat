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
            // Set the site's base URL so canonical links can use absolute URLs.
            .base_url("http://localhost:3000")
            .build(),
    )
    .await
    .unwrap();
}

// These values are defaults for all pages. Each page can replace individual fields.
#[layout(with(
    Metadata::new()
        .title("Acme")
        .description("Acme sells everything you need for tea.")
        .canonical(Url::Current)
        .theme_color("#0f766e")
        .open_graph(|og| og.site_name("Acme"))
        .twitter(|x| x.site("@acme"))
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

// Inherits every metadata field from the layout.
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

    // Overrides the title and omits the inherited description.
    #[page(with(Metadata::new().title("About").no_description()))]
    pub async fn page(cx: &Cx) -> Result<impl View> {
        // Read the combined metadata to use the same title in the page body.
        let metadata = metadata(cx).await?;

        Ok(view! {
            if let Some(title) = metadata.title() {
                <h1>(title)</h1>
            }
            <p>"This page has no description."</p>
        })
    }
}
