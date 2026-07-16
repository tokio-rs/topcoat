use topcoat_core::context::CxBuilder;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path, RouterBuilder};

use crate::{Config, OriginLayer, SessionState};

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

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        cx.insert(SessionState::new());
        next.run(cx, body)
    }
}

/// Installs session support on a [`RouterBuilder`].
pub trait RouterBuilderSessionExt {
    /// Registers the session `config` on the app context, the root session
    /// layer, and the [`OriginLayer`] rejecting state-changing cross-origin
    /// requests (unless disabled with
    /// [`Config::dangerous_disable_origin_verification`]).
    ///
    /// The default cookie token store also needs the cookie layer, registered
    /// with the cookie crate's `cookies` extension method.
    #[must_use]
    fn sessions(self, config: Config) -> Self;
}

impl RouterBuilderSessionExt for RouterBuilder {
    fn sessions(mut self, config: Config) -> Self {
        let verify_origin = config.verify_origin;
        self = self.app_context(config);
        self = self.layer(SessionLayer::new());
        if verify_origin {
            self = self.layer(OriginLayer::new());
        }
        self
    }
}
