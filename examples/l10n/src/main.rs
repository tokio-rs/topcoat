mod app;

#[tokio::main]
async fn main() {
    topcoat::start(app::router()).await.unwrap();
}
