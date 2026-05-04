use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use topcoat_router::Result;

use crate::{
    component::component,
    view::{View, view},
};

/// Notify the topcoat dev server that the application is ready.
///
/// Connects to the dev CLI's WebSocket server (address provided via
/// `TOPCOAT_DEV_URL` env var) and sends a `"ready"` message.
/// Does nothing if the env var is not set (i.e. not running under `topcoat dev`).
pub async fn notify_ready() {
    let Ok(url) = std::env::var("TOPCOAT_DEV_URL") else {
        return;
    };

    let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&url).await else {
        eprintln!("topcoat dev: failed to connect to {url}");
        return;
    };

    let _ = ws.send(Message::Text("ready".into())).await;
    let _ = ws.close(None).await;
}

#[component]
#[expect(
    unused_variables,
    reason = "child is required by the component macro contract but unused here"
)]
pub async fn script(child: View) -> Result {
    let url = std::env::var("TOPCOAT_DEV_URL").unwrap_or_default();

    view! {
        <script>
            (crate::view::Escaped::new_unchecked(format!(r#"
(function() {{
  function connect() {{
    var ws = new WebSocket("{url}");
    ws.onmessage = function(e) {{
      if (e.data === "reload") window.location.reload();
    }};
    ws.onclose = function() {{
      setTimeout(function() {{
        var retry = new WebSocket("{url}");
        retry.onopen = function() {{
          retry.close();
          window.location.reload();
        }};
        retry.onerror = function() {{ setTimeout(connect, 1000); }};
      }}, 500);
    }};
  }}
  connect();
}})();
"#)))
        </script>
    }
}
