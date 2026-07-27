use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    router::{
        Response, Router,
        content::websocket::{Message, WebSocketUpgrade},
        page, route,
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

// The page connects to the echo endpoint and logs each round trip.
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
                <script src=(asset!("./echo.js"))></script>
            </body>
        </html>
    }
}

// Upgrades the handshake and echoes every message back to the client.
#[route(GET "/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && socket.send(message).await.is_err()
            {
                break;
            }
        }
    })
}
