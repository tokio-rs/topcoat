// Nested components render concurrently, so a deeply composed page holds the
// render futures of its whole subtree at once.
#![allow(clippy::large_futures)]

mod app;
mod components;
mod customer;
mod models;

use toasty::Db;

#[tokio::main]
async fn main() {
    let mut db = Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await
        .unwrap();

    db.push_schema().await.unwrap();
    models::seed(&mut db).await.unwrap();

    topcoat::start(app::router(db)).await.unwrap();
}
