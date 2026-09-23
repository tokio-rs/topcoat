use topcoat_core::{context::Cx, error::Result};
use web_time::SystemTime;

use crate::{TokenHash, config, state, token::Token, token_store};

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
/// Call this after validating the current session. It reissues the token for
/// the configured [`lifetime`](crate::SessionConfigBuilder::lifetime) and
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
    token_store(cx)
        .write(cx, token, config(cx).lifetime)
        .await?;
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
        .write(cx, token.clone(), config(cx).lifetime)
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
    SystemTime::now() + config(cx).lifetime
}
