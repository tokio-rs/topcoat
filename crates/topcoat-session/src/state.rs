use tokio::sync::Mutex;
use topcoat_core::{
    context::{Cx, memoize_cache},
    error::Result,
};

use crate::{Token, token_store};

/// The current session token for one request.
///
/// It loads the token on first access and records session changes for later
/// reads.
#[derive(Debug, Default)]
pub(crate) struct SessionState {
    token: Mutex<Load>,
}

#[derive(Debug, Default)]
enum Load {
    #[default]
    Unloaded,
    Loaded(Option<Token>),
}

impl SessionState {
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

/// Returns the session state of the request, creating it on first access.
pub(crate) fn state(cx: &Cx) -> &SessionState {
    memoize_cache(cx).memoize(cx, (), (), |_, ()| SessionState::default())
}
