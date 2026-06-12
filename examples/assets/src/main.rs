use topcoat::{
    Result,
    asset::{AssetBundle, asset},
    router::{Router, page},
    view::view,
};

#[tokio::main]
async fn main() {
    let router = Router::new()
        .page(home)
        .assets(AssetBundle::load().unwrap());

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
                <img src=(asset!("./ferris.png"))>
            </body>
        </html>
    }
}
