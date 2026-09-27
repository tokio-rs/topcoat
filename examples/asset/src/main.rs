use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    router::{module_router, page},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    let router = module_router!()
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body>
                // The path is relative to this source file; the asset renders
                // as the bundled image's URL.
                <img src=(asset!("./ferris.png"))>
            </body>
        </html>
    })
}
