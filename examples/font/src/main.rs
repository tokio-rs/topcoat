use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

const LAVISHLY_YOURS: Font = fontsource_font!("Lavishly Yours", host: Asset);

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
                <link rel="stylesheet" href=(LAVISHLY_YOURS)>
            </head>
            <body>
                <h1 style=(format!("font-family: {:?}", LAVISHLY_YOURS.family()))>
                    "This font is downloaded from Fontsource and self-hosted via Topcoat assets!"
                </h1>
            </body>
        </html>
    }
}
