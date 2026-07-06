mod app;
mod catalog;
mod urls;

#[tokio::main]
async fn main() {
    topcoat::start(app::router()).await.unwrap();
}
