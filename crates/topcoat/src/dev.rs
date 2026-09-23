//! Integration with the `topcoat dev` server.
//!
//! Render [`script`] in the `<head>` of your pages to update them in the
//! browser whenever `topcoat dev` finishes a new build. Outside of
//! `topcoat dev`, it renders nothing.

#[cfg(feature = "serve")]
use std::net::SocketAddr;

#[cfg(feature = "serve")]
use futures_util::SinkExt;
#[cfg(feature = "serve")]
use tokio_tungstenite::tungstenite::Message;

use crate::{
    Result,
    view::{View, component, view},
};

/// Tells the `topcoat dev` server that the application is ready to accept
/// connections.
///
/// Pass the address the application listens on, if it has one. Does nothing
/// when the application is not running under `topcoat dev`.
///
/// [`serve`](crate::serve), [`serve_until`](crate::serve_until), and
/// [`start`](crate::start) call this for you. Call it yourself only when you
/// run the server another way.
#[cfg(feature = "serve")]
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

#[cfg(feature = "serve")]
fn http_to_ws(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if let Some(rest) = url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else {
        url.to_string()
    }
}

/// Renders the `topcoat dev` client script.
///
/// Place it in the `<head>` of your pages. Once `topcoat dev` serves a new
/// build, the script fetches the page again and merges the new HTML into it,
/// keeping matching elements and their form state. When the runtime is
/// loaded, signals whose identities still match keep their values too.
/// Moving signal calls or component invocations can change their identities.
///
/// Changes to scripts, the base URL, or the doctype trigger a full reload.
/// Reload manually to reset form state and signal values to their defaults.
///
/// A small floating status indicator shows rebuilds and failures. Pass
/// `status_indicator: false` to hide it while keeping live updates.
///
/// Renders nothing when the app is not running under `topcoat dev`.
#[component]
pub async fn script(#[default(true)] status_indicator: bool) -> Result<impl View> {
    let src = std::env::var("TOPCOAT_DEV_URL")
        .ok()
        .map(|base| format!("{base}/dev.js"));
    // Read by dev.js; only rendered when the indicator is disabled.
    let indicator_off = (!status_indicator).then_some("false");

    Ok(view! {
        if let Some(src) = src {
            <script src=(src) data-status-indicator=(indicator_off)></script>
        }
    })
}
