mod app;
mod components;
mod customer;
mod drinks;

#[tokio::main]
async fn main() {
    topcoat::start(app::router()).await.unwrap();
}
