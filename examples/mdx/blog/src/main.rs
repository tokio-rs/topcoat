mod app;

// --- Server ------------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(app::router()).await.unwrap();
}
