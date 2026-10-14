//! Topcoat keeps the session token in a cookie. The database stores its hash,
//! the user ID, and the expiry time.

use jiff::Timestamp;
use topcoat::{
    Result,
    context::{Cx, memoize},
    router::{
        error::{RouterErrorExt, internal_server_error},
        href,
    },
    session,
};

use super::user::User;
use crate::{app::login, db::db};

#[derive(Debug, toasty::Model)]
pub struct Session {
    /// Store only the token's SHA-256 hash, never the token itself.
    #[key]
    token_hash: Vec<u8>,

    #[index]
    user_id: u64,

    #[belongs_to]
    user: toasty::Deferred<User>,

    expires_at: Timestamp,
}

pub async fn sign_in(cx: &Cx, user: &User) -> Result<()> {
    let session = session::start(cx).await?;
    let expires_at = Timestamp::try_from(session.expires_at).map_err(internal_server_error)?;

    toasty::create!(Session {
        token_hash: session.token_hash.to_vec(),
        user_id: user.id,
        expires_at,
    })
    .exec(&mut db(cx))
    .await?;

    Ok(())
}

pub async fn sign_out(cx: &Cx) -> Result<()> {
    if let Some(token_hash) = session::stop(cx).await? {
        Session::filter_by_token_hash(token_hash.to_vec())
            .delete()
            .exec(&mut db(cx))
            .await?;
    }
    Ok(())
}

/// Returns the signed-in user, or `None` if the session is missing or expired.
/// The result is cached for the request.
#[memoize(as_ref)]
pub async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(token_hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };

    let mut db = db(cx);
    let session = Session::filter_by_token_hash(token_hash.to_vec())
        .first()
        .exec(&mut db)
        .await?;

    let Some(session) = session.filter(|session| session.expires_at > Timestamp::now()) else {
        return Ok(None);
    };

    let user = User::get_by_id(&mut db, &session.user_id).await?;
    Ok(Some(user))
}

/// Requires a valid session and redirects to sign-in if there is none.
/// Call this from protected pages, shards, and procedures. Repeated calls share
/// the cached result from [`current_user`].
pub async fn require_user(cx: &Cx) -> Result<&User> {
    let user = current_user(cx).await?;
    Ok(user
        .as_ref()
        .ok_or_redirect(href!(login::page).resolve(cx))?)
}
