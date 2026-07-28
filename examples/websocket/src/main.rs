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
    // Register the page and WebSocket endpoint, load the generated assets,
    // and start the server at http://127.0.0.1:3000 by default.
    let router = Router::builder()
        .page(home)
        .route(echo)
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

// Render the page that connects to the WebSocket echo endpoint.
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

                // Messages submitted through this form are sent over
                // the WebSocket connection.
                <form id="form">
                    <input id="input" autocomplete="off" placeholder="Say something">
                    <button>"Send"</button>
                </form>

                // Connection events and echoed messages are added here.
                <ul id="log"></ul>

                // Load the browser code that manages the WebSocket connection.
                <script src=(asset!("./echo.js"))></script>
            </body>
        </html>
    }
}

// Upgrade GET /echo from HTTP to WebSocket and echo each text or binary
// message back to the connected client.
#[route(GET "/echo")]
async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
    upgrade.on_upgrade(|mut socket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            // Ping, pong, and close messages are handled by the WebSocket
            // implementation. This example echoes text and binary messages.
            if matches!(message, Message::Text(_) | Message::Binary(_))
                && socket.send(message).await.is_err()
            {
                break;
            }
        }
    })
}
