use topcoat_core::{context::Cx, error::Result};
use web_time::SystemTime;

use crate::{TokenHash, config, state, token::Token, token_store};

/// The data the application saves for a session.
///
/// Returned by [`start`], [`refresh`], and [`rotate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The hash that identifies the session. Save it together with the user
    /// the session authenticates.
    pub token_hash: TokenHash,
    /// When the session expires. Save it with the hash and treat the session
    /// as invalid after this time.
    pub expires_at: SystemTime,
}

/// The result of [`rotate`]: the hash of the old token and the new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotation {
    /// The hash of the replaced token. Delete its record, or move the record
    /// to the hash of the new session.
    pub revoked: TokenHash,
    /// The new session to save.
    pub session: Session,
}

/// Starts a new session and sends its token to the client.
///
/// Always creates a new token and never reuses the one the request carried,
/// so calling this on login protects against session fixation. Save the
/// returned [`Session`] in the application's session storage.
///
/// # Errors
///
/// Returns an error when the token store fails to send the token.
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

/// Stops the current session and tells the client to discard its token.
///
/// Returns the hash of the stopped session so the application can delete its
/// record, or `None` when the request carried no token.
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

/// Extends the current session by a full
/// [`lifetime`](crate::SessionConfigBuilder::lifetime) without changing its
/// token.
///
/// Call this on use for sliding expiration. Returns the session with its new
/// expiry time so the application can update its record, or `None` when the
/// request carried no token.
///
/// # Errors
///
/// Returns an error when the token store fails to read or send the token.
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

/// Replaces the current session's token with a new one.
///
/// Rotate after a privilege change, or periodically, so a token that leaked
/// earlier stops working. Returns a [`Rotation`] with the hash of the old
/// token and the new session to save, or `None` when the request carried no
/// token.
///
/// # Errors
///
/// Returns an error when the token store fails to read or send a token.
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

/// Returns the hash of the current request's token, or `None` when the request
/// carries no valid token.
///
/// Look up the hash in the application's session storage. A hash the storage
/// does not contain, or whose record has expired, is not a valid session.
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
