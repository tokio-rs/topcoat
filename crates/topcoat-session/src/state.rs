use tokio::sync::Mutex;
use topcoat_core::{
    context::{Cx, request_context},
    error::Result,
};

use crate::{Token, token_store};

/// The request-scoped session cell, registered on the request context by the
/// session layer (or manually via `CxTestBuilder` in tests).
///
/// The cell caches the token presented by the request so the token store is
/// read at most once per request, and it is updated by the lifecycle
/// functions ([`start`](crate::start), [`stop`](crate::stop),
/// [`rotate`](crate::rotate)) so later reads within the same request observe
/// the change.
#[derive(Debug, Default)]
pub struct SessionState {
    token: Mutex<Load>,
}

#[derive(Debug, Default)]
enum Load {
    #[default]
    Unloaded,
    Loaded(Option<Token>),
}

impl SessionState {
    /// Creates an empty cell; the token is loaded on first access.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the request's current token, reading the token store on first
    /// access. Concurrent callers share the one read.
    pub(crate) async fn token(&self, cx: &Cx) -> Result<Option<Token>> {
        let mut load = self.token.lock().await;
        if let Load::Loaded(token) = &*load {
            return Ok(token.clone());
        }
        let token = token_store(cx).read(cx).await?;
        *load = Load::Loaded(token.clone());
        Ok(token)
    }

    /// Replaces the request's view of the token after a lifecycle change.
    pub(crate) async fn set(&self, token: Option<Token>) {
        *self.token.lock().await = Load::Loaded(token);
    }
}

pub(crate) fn state(cx: &Cx) -> &SessionState {
    request_context(cx)
}
