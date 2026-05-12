use std::path::PathBuf;

use topcoat::asset::AssetBundle;

mod app;
mod components;

#[tokio::main]
async fn main() {
    let router = app::router()
        .assets(AssetBundle::load(PathBuf::from("../../target/assets")).unwrap())
        .app_state(5);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::serve(listener, router).await.unwrap();
}
