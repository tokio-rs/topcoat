use topcoat_core::{context::Cx, error::Result};
use web_time::SystemTime;

use crate::{TokenHash, lifetime, state::state, token::Token, token_store};

/// A session record to persist in application storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The token hash. Store it alongside the user ID, without the raw token.
    pub token_hash: TokenHash,
    /// The expiry time. Reject the session after this time.
    pub expires_at: SystemTime,
}

/// The outcome of [`rotate`]: the replacement session and the hash it
/// replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotation {
    /// The hash of the replaced token. Delete its record, or re-key the
    /// record under the new session's hash.
    pub revoked: TokenHash,
    /// The replacement session to record.
    pub session: Session,
}

/// Starts a new session, issuing a fresh token to the client.
///
/// Store the returned [`Session`] after authenticating the user. The token is
/// always new, which protects against session fixation.
///
/// # Errors
///
/// Returns an error when the token store fails to issue the token.
pub async fn start(cx: &Cx) -> Result<Session> {
    let token = Token::random();
    let session = Session {
        token_hash: token.hash(),
        expires_at: expires_at(cx),
    };
    token_store(cx)
        .write(cx, token.clone(), lifetime(cx))
        .await?;
    state(cx).set(Some(token)).await;
    Ok(session)
}

/// Stops the current session, instructing the client to discard its token.
///
/// Returns the hash of the stopped session so the application can delete its
/// record, or `None` when the request carried no session.
///
/// # Errors
///
/// Returns an error when the token store fails to read or discard the token.
pub async fn stop(cx: &Cx) -> Result<Option<TokenHash>> {
    let hash = state(cx).token(cx).await?.map(|token| token.hash());
    token_store(cx).delete(cx).await?;
    state(cx).set(None).await;
    Ok(hash)
}

/// Extends the current session's lifetime without changing its token.
///
/// Call this after validating the current session. It reissues the token for
/// the configured [`SessionLifetime`](crate::SessionLifetime) and
/// returns the new expiry for you to store. Returns `None` if no token is
/// available.
///
/// # Errors
///
/// Returns an error when the token store fails to read or re-issue the token.
pub async fn refresh(cx: &Cx) -> Result<Option<Session>> {
    let Some(token) = state(cx).token(cx).await? else {
        return Ok(None);
    };
    let session = Session {
        token_hash: token.hash(),
        expires_at: expires_at(cx),
    };
    token_store(cx).write(cx, token, lifetime(cx)).await?;
    Ok(Some(session))
}

/// Replaces the current session's token with a fresh one.
///
/// Returns a [`Rotation`] describing the old record to revoke and the new
/// session to store, or `None` if no token is available. Revoke the old record
/// to stop the old token from authenticating. Use this after a privilege change.
///
/// # Errors
///
/// Returns an error when the token store fails to read or issue a token.
pub async fn rotate(cx: &Cx) -> Result<Option<Rotation>> {
    let Some(old) = state(cx).token(cx).await? else {
        return Ok(None);
    };
    let token = Token::random();
    let rotation = Rotation {
        revoked: old.hash(),
        session: Session {
            token_hash: token.hash(),
            expires_at: expires_at(cx),
        },
    };
    token_store(cx)
        .write(cx, token.clone(), lifetime(cx))
        .await?;
    state(cx).set(Some(token)).await;
    Ok(Some(rotation))
}

/// Returns the current token's hash, or `None` if no token is available.
///
/// Look up the hash in application storage to authenticate the session.
/// Reject missing or expired records. This function does not validate the
/// stored session.
///
/// # Errors
///
/// Returns an error when the token store fails to read the token.
pub async fn token_hash(cx: &Cx) -> Result<Option<TokenHash>> {
    Ok(state(cx).token(cx).await?.map(|token| token.hash()))
}

fn expires_at(cx: &Cx) -> SystemTime {
    SystemTime::now() + lifetime(cx)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::*;
    use crate::{SessionLifetime, SessionTokenStore, TokenStore, TokenStoreFuture};

    /// Presents `token` on every read and records the store's calls.
    #[derive(Clone)]
    struct FixedTokenStore {
        token: Token,
        reads: Arc<Mutex<usize>>,
        writes: Arc<Mutex<Vec<Duration>>>,
    }

    impl FixedTokenStore {
        fn new() -> Self {
            Self {
                token: Token::random(),
                reads: Arc::default(),
                writes: Arc::default(),
            }
        }
    }

    impl TokenStore for FixedTokenStore {
        fn read<'a>(&'a self, _cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
            *self.reads.lock().unwrap() += 1;
            Box::pin(async move { Ok(Some(self.token.clone())) })
        }

        fn write<'a>(
            &'a self,
            _cx: &'a Cx,
            _token: Token,
            max_age: Duration,
        ) -> TokenStoreFuture<'a, ()> {
            self.writes.lock().unwrap().push(max_age);
            Box::pin(async { Ok(()) })
        }

        fn delete<'a>(&'a self, _cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
            Box::pin(async { Ok(()) })
        }
    }

    /// Child contexts of one request must see the same session, so a session
    /// started in one is visible in another and the token is read only once.
    #[tokio::test]
    async fn child_contexts_share_the_session_of_the_request() -> Result<()> {
        let store = FixedTokenStore::new();
        let cx = Cx::new().with(SessionTokenStore::new(store.clone()));

        let presented = token_hash(&cx.with(1_u8)).await?;
        assert_eq!(presented, Some(store.token.hash()));

        let started = start(&cx.with(2_u8)).await?;
        assert_eq!(token_hash(&cx).await?, Some(started.token_hash));
        assert_eq!(*store.reads.lock().unwrap(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn issued_tokens_use_the_lifetime_in_the_context() -> Result<()> {
        let store = FixedTokenStore::new();
        let lifetime = Duration::from_hours(2);
        let cx = Cx::new()
            .with(SessionTokenStore::new(store.clone()))
            .with(SessionLifetime(lifetime));

        let before = SystemTime::now();
        let session = start(&cx).await?;
        refresh(&cx).await?;

        assert_eq!(*store.writes.lock().unwrap(), [lifetime, lifetime]);
        assert!(session.expires_at >= before + lifetime);
        assert!(session.expires_at < before + SessionLifetime::DEFAULT.0);
        Ok(())
    }
}
