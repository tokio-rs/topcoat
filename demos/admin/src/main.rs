mod app;
// Registry components are copied in full, including parts this demo does not use.
#[allow(dead_code)]
mod components;
mod db;
mod features;
mod seed;

#[tokio::main]
async fn main() {
    let db = db::connect().await.unwrap();

    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "toasty") {
        if let Err(error) = db::cli(db).parse_from(args).await {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }

    db::migrate(&db).await.unwrap();
    seed::seed(&db).await.unwrap();

    topcoat::start(app::router().app_context(db).build())
        .await
        .unwrap();
}
