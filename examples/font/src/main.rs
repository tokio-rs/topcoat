use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, font, fontsource},
    router::{Router, page},
    view::view,
};

static FONT: Font = Font::new(
    fontsource::LAVISHLY_YOURS.name,
    &[fontsource::fontsource_font_face!(
        fontsource::LAVISHLY_YOURS,
        weight: 400,
        style: fontsource::Style::Normal,
        subset: fontsource::Subset::Latin,
        host: asset,
    )],
);

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .page(home)
        .assets(AssetBundle::load().unwrap())
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
            </head>
            <body>
            </body>
        </html>
    }
}
