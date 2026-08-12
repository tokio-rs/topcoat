use topcoat_core::context::Cx;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path, RouterBuilder};

use crate::{SessionConfig, SessionState};

/// A router layer that makes the session state available for the current
/// request.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionLayer;

impl SessionLayer {
    /// Creates a session layer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Layer for SessionLayer {
    fn path(&self) -> &Path {
        Path::new("/")
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        let cx = cx.with(SessionState::new());
        Box::pin(async move { next.run(&cx, body).await })
    }
}

/// Installs session support on a [`RouterBuilder`].
pub trait RouterBuilderSessionExt {
    /// Registers the session `config` on the app context and the root session
    /// layer.
    ///
    /// The default cookie token store also needs the cookie layer, registered
    /// with the cookie crate's `cookies` extension method.
    #[must_use]
    fn sessions(self, config: SessionConfig) -> Self;
}

impl RouterBuilderSessionExt for RouterBuilder {
    fn sessions(mut self, config: SessionConfig) -> Self {
        self = self.app_context(config);
        self = self.layer(SessionLayer::new());
        self
    }
}
