mod app;
mod components;
mod customer;
mod drinks;

use toasty::Db;

#[tokio::main]
async fn main() {
    let mut db = Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await
        .unwrap();

    db.push_schema().await.unwrap();
    drinks::seed(&mut db).await.unwrap();

    topcoat::start(app::router(db)).await.unwrap();
}
