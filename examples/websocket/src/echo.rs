use topcoat::{Result, router::{content::websocket::{Message, WebSocketUpgrade}, response::Response, route}};

#[route(GET)]
pub(crate) async fn echo(upgrade: WebSocketUpgrade) -> Result<Response> {
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
