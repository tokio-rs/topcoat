mod app;

#[tokio::main]
async fn main() {
    let router = app::router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    topcoat::serve(listener, router).await.unwrap();
}
