use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    router::{
        Router,
        content::websocket::{Message, WebSocketUpgrade},
        page,
        response::Response,
        route,
    },
    view::view,
};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .page(home)
        .route(echo)
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"WebSocket echo"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"WebSocket echo"</h1>

                <form id="form">
                    <input id="input" autocomplete="off" placeholder="Say something">
                    <button>"Send"</button>
                </form>

                <ul id="log"></ul>

                // Opens the connection and logs what is sent and received.
                <script src=(asset!("./echo.js"))></script>
            </body>
        </html>
    }
}

#[route(GET "/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            // Ping, pong, and close messages are already handled for us.
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && socket.send(message).await.is_err()
            {
                break;
            }
        }
    })
}
