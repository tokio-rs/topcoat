mod app;
mod components;
mod draft;
mod models;

#[tokio::main]
async fn main() {
    let mut db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await
        .unwrap();
    db.push_schema().await.unwrap();
    models::seed(&mut db).await.unwrap();
    topcoat::start(app::router(db)).await.unwrap();
}
