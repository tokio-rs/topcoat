use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// The simple way: let the `fontsource_font!` macro pick the faces out of the
// Fontsource (Google Fonts) catalog. `host: Asset` downloads the files and
// self-hosts them as Topcoat assets.
const LAVISHLY_YOURS: Font = fontsource_font!("Lavishly Yours", host: Asset);

// The manual way: declare the `@font-face` rules by hand with the `font!` macro,
// in this case pointing straight at a font on the jsDelivr CDN. You can also use
// the Topcoat asset system here by wrapping the URL in `asset!(...)`.
const ORBITRON: Font = font! {
    "Orbitron",
    @font-face {
        src: url("https://cdn.jsdelivr.net/fontsource/fonts/orbitron:vf@latest/latin-wght-normal.woff2") format("woff2") tech("variations");
        font-weight: 100 900;
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
                topcoat::dev::script()
                // The `link` component preloads fonts efficiently by default.
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
