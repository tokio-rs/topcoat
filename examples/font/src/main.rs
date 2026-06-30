use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

const FONT: Font = fontsource_font!("Lavishly Yours", host: asset);

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
                <link rel="stylesheet" href=(FONT)>
            </head>
            <body>
                <h1 style=(format!("font-family: {:?}", FONT.family()))>
                    "Lavishly Yours"
                </h1>
            </body>
        </html>
    }
}
