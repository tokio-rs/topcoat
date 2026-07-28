use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    router::{Router, page},
    view::view,
};

#[tokio::main]
async fn main() {
    // Load the generated asset bundle, register the page, and start the server.
    // By default, the application is available at http://127.0.0.1:3000.
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
            <head>topcoat::dev::script()</head>
            <body>
                // `asset!` declares a file relative to this Rust source file.
                // Topcoat replaces it with the URL of the bundled image.
                <img src=(asset!("./ferris.png"))>
            </body>
        </html>
    }
}
