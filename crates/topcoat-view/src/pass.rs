use topcoat_core::context::{Cx, try_request_context};

/// Which pass a view is being rendered in.
///
/// On the first render, a view may suspend and render skeletons.
/// However, the goal is to finish an initial representation of the
/// page as quickly as possible. This is the `Initial` pass.
/// It must terminate, such that the browser's loading spinner stops spinning,
/// and the page is interpreted as "fully loaded".
///
/// Afterwards, if the view contains `live!` regions that need to be
/// kept up to date by the server, the client must reconnect to the
/// server via a WebSocket connection. Once this connection is established,
/// the server can send as many update events as desired. This is the
/// second, `Connected` pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pass {
    #[default]
    Initial,
    Connected,
}

/// Which render pass the current request is rendering.
///
/// On the first render, a view may suspend and render skeletons.
/// However, the goal is to finish an initial representation of the
/// page as quickly as possible. This is the `Initial` pass.
/// It must terminate, such that the browser's loading spinner stops spinning,
/// and the page is interpreted as "fully loaded".
///
/// Afterwards, if the view contains `live!` regions that need to be
/// kept up to date by the server, the client must reconnect to the
/// server via a WebSocket connection. Once this connection is established,
/// the server can send as many update events as desired. This is the
/// second, `Connected` pass.
#[must_use]
pub fn pass(cx: &Cx) -> Pass {
    try_request_context(cx).copied().unwrap_or_default()
}
