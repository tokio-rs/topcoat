use crate::Result;
use futures_util::SinkExt;
use std::net::SocketAddr;
use tokio_tungstenite::tungstenite::Message;

use crate::view::{component, view};

/// Notify the topcoat dev server that the application is ready.
///
/// Connects to the dev server's WebSocket endpoint (derived from the
/// `TOPCOAT_DEV_URL` HTTP base URL provided by `topcoat dev`) and sends a
/// ready message with the application listener address when available. Does
/// nothing if the env var is not set.
pub async fn notify_ready(addr: Option<SocketAddr>) {
    let Ok(base) = std::env::var("TOPCOAT_DEV_URL") else {
        return;
    };

    let ws_url = http_to_ws(&base) + "/ws";

    let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&ws_url).await else {
        eprintln!("topcoat dev: failed to connect to {ws_url}");
        return;
    };

    let text = match addr {
        Some(addr) => format!("ready {addr}"),
        None => "ready".to_owned(),
    };

    let _ = ws.send(Message::Text(text.into())).await;
    let _ = ws.close(None).await;
}

fn http_to_ws(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if let Some(rest) = url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else {
        url.to_string()
    }
}

/// Inject the `topcoat dev` client script.
///
/// The script reloads the page once a new build is serving, and shows a
/// small floating status indicator while the dev server is rebuilding or
/// after a build failure. Pass `status_indicator: false` to disable the
/// indicator while keeping live reload.
///
/// Renders nothing when the app is not running under `topcoat dev`.
#[component]
pub async fn script(#[default(true)] status_indicator: bool) -> Result {
    let Ok(base) = std::env::var("TOPCOAT_DEV_URL") else {
        return view! {};
    };
    let src = format!("{base}/dev.js");
    // Read by dev.js; only rendered when the indicator is disabled.
    let indicator_off = (!status_indicator).then_some("false");

    view! { <script src=(src) data-status-indicator=(indicator_off)></script> }
}
