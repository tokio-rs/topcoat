use tokio::sync::Mutex;
use topcoat_core::{
    context::{Cx, request_context},
    error::Result,
};

use crate::{Token, token_store};

/// Request context slot that holds the session token of the current request.
///
/// The session router layer adds one to the request context of every request.
/// You only need to create one yourself when you build a
/// [`Cx`] by hand, for example in tests.
///
/// The token is read from the token store at most once per request. The
/// lifecycle functions, such as [`start`](crate::start) and
/// [`stop`](crate::stop), update it, so later code in the same request sees
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
    /// Creates an empty slot. The token is read on first use.
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
