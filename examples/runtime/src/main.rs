use topcoat::{
    Result,
    asset::AssetBundle,
    router::{Router, page},
    view::view,
};

#[tokio::main]
async fn main() {
    topcoat::start(
        Router::new()
            .assets(AssetBundle::load().unwrap())
            .discover(),
    )
    .await
    .unwrap();
}

#[page("/")]
async fn home() -> Result {
    let x = 6.0;
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Hello world"</title>
                <script type="module" src=(topcoat::runtime::SCRIPT)></script>
                topcoat::dev::script()
            </head>
            <body>
                <button @click=$(|_e| {
                    raw!("console.log(${x})")
                })>"press me"</button>
            </body>
        </html>
    }
}
