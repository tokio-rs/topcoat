mod app;

#[tokio::main]
async fn main() {
    // Build the router from the module structure declared under `app`.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(app::router()).await.unwrap();
}
