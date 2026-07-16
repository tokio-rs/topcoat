use std::time::SystemTime;

use topcoat_core::{context::Cx, error::Result};

use crate::{TokenHash, config, state, token::Token, token_store};

/// A session as the application should record it, returned by [`start`],
/// [`refresh`], and [`rotate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The hash identifying the session. Persist it next to the user it
    /// authenticates; the raw token never needs to be stored server-side.
    pub token_hash: TokenHash,
    /// When the session expires. Persist it with the hash and reject the
    /// session once the moment has passed.
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
/// Always generates a new token (never reuses one the request presented), so
/// calling this on login also protects against session fixation. Record the
/// returned [`Session`] in the application's session storage.
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
        .write(cx, token.clone(), config(cx).lifetime)
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
/// Re-issues the presented token with a full [`lifetime`](crate::ConfigBuilder::lifetime)
/// ahead of it, implementing sliding expiration. Returns the session with its
/// new expiry so the application can update its record, or `None` when the
/// request carried no session.
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
    token_store(cx)
        .write(cx, token, config(cx).lifetime)
        .await?;
    Ok(Some(session))
}

/// Replaces the current session's token with a fresh one.
///
/// Rotate after a privilege change (or periodically) so a previously leaked
/// token stops working. Returns the [`Rotation`] describing the record to
/// revoke and the session to record in its place, or `None` when the request
/// carried no session.
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
        .write(cx, token.clone(), config(cx).lifetime)
        .await?;
    state(cx).set(Some(token)).await;
    Ok(Some(rotation))
}

/// Returns the hash identifying the current request's session, or `None`
/// when the request carries no (valid) token.
///
/// Look the hash up in the application's session storage to resolve the
/// session; a hash the storage does not contain (or whose record has
/// expired) is not an authenticated session.
///
/// # Errors
///
/// Returns an error when the token store fails to read the token.
pub async fn token_hash(cx: &Cx) -> Result<Option<TokenHash>> {
    Ok(state(cx).token(cx).await?.map(|token| token.hash()))
}

fn expires_at(cx: &Cx) -> SystemTime {
    SystemTime::now() + config(cx).lifetime
}
