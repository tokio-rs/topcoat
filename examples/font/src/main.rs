use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// `host: Asset` downloads the font files into the asset bundle, so Topcoat
// serves them itself.
const LAVISHLY_YOURS: Font = fontsource_font!(LAVISHLY_YOURS, host: Asset);

// A font declared by hand. The source can be any URL, a local file, or an
// asset; this one stays on the CDN.
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

                // Renders the preload and style elements the font needs.
                topcoat::font::link(font: LAVISHLY_YOURS)
                topcoat::font::link(font: ORBITRON)
            </head>

            <body>
                <h1 style=(format!("font-family: {:?}", LAVISHLY_YOURS.family()))>
                    "This font is downloaded from Fontsource and self-hosted via Topcoat assets!"
                </h1>

                <h2 style=(format!("font-family: {:?}", ORBITRON.family()))>
                    "This font is declared by hand and loaded straight from the jsDelivr CDN!"
                </h2>
            </body>
        </html>
    }
}
