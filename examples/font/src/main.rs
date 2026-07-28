use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// Select Lavishly Yours from the Fontsource catalog.
//
// `host: Asset` downloads the font files while building the asset bundle
// and serves them locally through Topcoat.
const LAVISHLY_YOURS: Font = fontsource_font!(LAVISHLY_YOURS, host: Asset);

// Declare a font manually with an `@font-face` rule.
//
// This example loads the variable Orbitron font directly from jsDelivr.
// A local file or a Topcoat asset could also be used as the source.
const ORBITRON: Font = font! {
    "Orbitron",
    @font-face {
        src: url(
            "https://cdn.jsdelivr.net/fontsource/fonts/orbitron:vf@latest/latin-wght-normal.woff2"
        ) format("woff2") tech("variations");
        font-weight: 100 900;
        font-display: swap;
    }
};

#[tokio::main]
async fn main() {
    // Load the generated assets, discover the page, and start the server.
    // By default, the application is available at http://127.0.0.1:3000.
    let router = Router::builder()
        .assets(AssetBundle::load().unwrap())
        .discover()
        .build();

    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Fonts"</title>
                topcoat::dev::script()

                // Generate the required preload and font style elements.
                topcoat::font::link(font: LAVISHLY_YOURS)
                topcoat::font::link(font: ORBITRON)
            </head>

            <body>
                // Use the self-hosted Fontsource font.
                <h1 style=(format!("font-family: {:?}", LAVISHLY_YOURS.family()))>
                    "This font is downloaded from Fontsource and self-hosted via Topcoat assets!"
                </h1>

                // Use the manually declared font loaded from jsDelivr.
                <h2 style=(format!("font-family: {:?}", ORBITRON.family()))>
                    "This font is declared by hand and loaded straight from the jsDelivr CDN!"
                </h2>
            </body>
        </html>
    }
}
