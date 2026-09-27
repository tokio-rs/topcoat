use topcoat::asset::RouterBuilderAssetExt;

mod echo;

use topcoat::{
    Result,
    asset::{AssetBundle, asset},
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
            <head>
                <title>"WebSocket echo"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"WebSocket echo"</h1>

                <form id="form">
                    <input id="input" autocomplete="off" placeholder="Say something">
                    <button>"Send"</button>
                </form>

                <ul id="log"></ul>

                // Opens the connection and logs what is sent and received.
                <script src=(asset!("./echo.js"))></script>
            </body>
        </html>
    })
}
