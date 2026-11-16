use std::fmt;

use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};
use web_time::SystemTime;

use crate::{TokenHash, TokenStore, state::state, time_to_live, token::Token, token_store};

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

/// A session token that has been generated but not yet issued to the
/// client.
///
/// [`prepare_start`] and [`prepare_rotate`] return one. Store the record
/// described by [`session`](Self::session), then call
/// [`commit`](Self::commit) to issue the token. Dropping a `Pending` issues
/// nothing: the client keeps its current token and the request's view of the
/// session is unchanged. This lets a request whose own write fails leave the
/// client's session untouched.
#[must_use = "a pending session issues nothing until committed"]
pub struct Pending<'a> {
    cx: &'a Cx,
    store: &'a dyn TokenStore,
    token: Token,
    session: Session,
}

impl fmt::Debug for Pending<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pending")
            .field("cx", &self.cx)
            .field("token", &self.token)
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}

impl Pending<'_> {
    /// The session to record: the hash to store and the expiry to keep.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Issues the token to the client and updates the request's view of the
    /// session, returning the issued [`Session`].
    ///
    /// The returned [`Session::expires_at`] was fixed when the token was
    /// prepared, while the token store's time to live starts now, so the
    /// record expires no later than the client's token.
    ///
    /// A `commit` dropped before it completes may already have issued the
    /// token; the request's view of the session is then not updated.
    ///
    /// # Errors
    ///
    /// Returns a [`CommitError`] when the token store fails to issue the
    /// token. The request's view of the session is left unchanged; see
    /// [`CommitError`] for what the client holds and which record to delete.
    ///
    /// The store's own error is the [`source`](std::error::Error::source) of
    /// the `CommitError`. Converting the `CommitError` with `?` hides that
    /// source from [`Error::is`] and [`Error::downcast_ref`], which look through
    /// context layers only. A handler that maps a custom store's error by
    /// type should take it out with [`into_parts`](CommitError::into_parts)
    /// first.
    ///
    /// # Panics
    ///
    /// Panics when the token store's [`write`](crate::TokenStore::write) does.
    /// The default cookie store panics once the response headers have been
    /// sent. A panic leaves the record you stored in place with no token
    /// issued and no [`CommitError`] to name it, so commit before the response
    /// starts.
    pub async fn commit(self) -> Result<Session, CommitError> {
        let Self {
            cx,
            store,
            token,
            session,
        } = self;
        if let Err(source) = store.write(cx, token.clone(), time_to_live(cx)).await {
            return Err(CommitError { session, source });
        }
        state(cx).set(Some(token)).await;
        Ok(session)
    }
}

/// A session token the token store could not issue.
///
/// The client keeps its old token unless the store failed after sending the
/// new one, so it may hold either. If the application already moved its
/// record to the new hash, delete the record under
/// [`session`](Self::session): a client that received the new token stays
/// signed in until that record is gone.
#[derive(Debug, thiserror::Error)]
#[error("failed to issue the session token")]
pub struct CommitError {
    session: Session,
    #[source]
    source: Error,
}

impl CommitError {
    /// The session whose token the store failed to issue.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Splits the error into the unissued session and the token store's
    /// error, for callers that handle the store's error by type or own the
    /// session.
    #[must_use]
    pub fn into_parts(self) -> (Session, Error) {
        (self.session, self.source)
    }
}

/// The outcome of [`prepare_rotate`]: the hash to revoke and the pending
/// replacement.
#[must_use = "a pending rotation issues nothing until committed"]
#[derive(Debug)]
#[non_exhaustive]
pub struct PendingRotation<'a> {
    /// The hash of the token the request currently holds. Delete its record,
    /// or re-key the record under the pending session's hash.
    pub revoked: TokenHash,
    /// The replacement to commit once the record has moved.
    pub pending: Pending<'a>,
}

/// Prepares a new session without issuing its token yet.
///
/// Store the [`session`](Pending::session) of the returned [`Pending`] after
/// authenticating the user, then [`commit`](Pending::commit) the token. The
/// token is always new, which protects against session fixation. Use
/// [`start`] when the record can be stored after the token is issued.
///
/// To revoke a session the request arrived with, read its hash with
/// [`token_hash`] first.
///
/// # Panics
///
/// Panics when the platform's source of randomness fails, as
/// [`Token::random`] does, or when the
/// [`SessionLifetime`](crate::SessionLifetime) is too large to add to the
/// current time, or when the context has no token store; see
/// [`SessionTokenStore`](crate::SessionTokenStore).
pub fn prepare_start(cx: &Cx) -> Pending<'_> {
    let store = token_store(cx);
    let token = Token::random();
    let session = new_session(cx, &token);
    Pending {
        cx,
        store,
        token,
        session,
    }
}

/// Starts a new session, issuing a fresh token to the client.
///
/// Store the returned [`Session`] after authenticating the user. The token is
/// always new, which protects against session fixation. To store the record
/// before the token is issued, use [`prepare_start`].
///
/// # Panics
///
/// Panics as [`prepare_start`] does, and when the token store's
/// [`write`](crate::TokenStore::write) does.
///
/// # Errors
///
/// Returns an error when the token store fails to issue the token.
pub async fn start(cx: &Cx) -> Result<Session> {
    prepare_start(cx)
        .commit()
        .await
        .map_err(|error| error.source)
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
    let session = new_session(cx, &token);
    token_store(cx).write(cx, token, time_to_live(cx)).await?;
    Ok(Some(session))
}

/// Replaces the current session's token with a fresh one.
///
/// Returns a [`Rotation`] describing the old record to revoke and the new
/// session to store, or `None` if no token is available. Revoke the old record
/// to stop the old token from authenticating. Use this after a privilege
/// change. To move the record before the token is issued, use
/// [`prepare_rotate`].
///
/// # Panics
///
/// Panics as [`prepare_start`] does, and when the token store's
/// [`write`](crate::TokenStore::write) does.
///
/// # Errors
///
/// Returns an error when the token store fails to read or issue a token.
pub async fn rotate(cx: &Cx) -> Result<Option<Rotation>> {
    let Some(PendingRotation { revoked, pending }) = prepare_rotate(cx).await? else {
        return Ok(None);
    };
    let session = pending.commit().await.map_err(|error| error.source)?;
    Ok(Some(Rotation { revoked, session }))
}

/// Prepares a replacement for the current session's token without issuing
/// it yet.
///
/// Returns `None` if no token is available. Otherwise move the record from
/// [`revoked`](PendingRotation::revoked) to the pending session's hash, then
/// [`commit`](Pending::commit) the token. Use this after a privilege change
/// whose record write can fail or race, so a request that achieves nothing
/// leaves the client's token untouched. Use [`rotate`] when the record can be
/// moved after the token is issued.
///
/// `revoked` is the hash of the token the request holds at the time of the
/// call. Two uncommitted rotations in one request name the same `revoked`;
/// the last commit wins.
///
/// # Panics
///
/// Panics as [`prepare_start`] does.
///
/// # Errors
///
/// Returns an error when the token store fails to read the token.
pub async fn prepare_rotate(cx: &Cx) -> Result<Option<PendingRotation<'_>>> {
    let Some(old) = state(cx).token(cx).await? else {
        return Ok(None);
    };
    Ok(Some(PendingRotation {
        revoked: old.hash(),
        pending: prepare_start(cx),
    }))
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

/// The record for `token` issued now, for the lifetime in the context.
fn new_session(cx: &Cx, token: &Token) -> Session {
    Session {
        token_hash: token.hash(),
        expires_at: SystemTime::now() + time_to_live(cx),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::*;
    use crate::{SessionLifetime, SessionTokenStore, TokenStoreFuture};

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
            time_to_live: Duration,
        ) -> TokenStoreFuture<'a, ()> {
            self.writes.lock().unwrap().push(time_to_live);
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

    /// A cookie's `Max-Age` carries whole seconds, so a fractional lifetime is
    /// rounded up for the token store and the record alike.
    #[tokio::test]
    async fn a_fractional_lifetime_is_rounded_up_to_whole_seconds() -> Result<()> {
        let store = FixedTokenStore::new();
        let cx = Cx::new()
            .with(SessionTokenStore::new(store.clone()))
            .with(SessionLifetime(Duration::from_millis(1100)));

        let before = SystemTime::now();
        let session = start(&cx).await?;

        assert_eq!(*store.writes.lock().unwrap(), [Duration::from_secs(2)]);
        assert!(session.expires_at >= before + Duration::from_secs(2));
        Ok(())
    }

    fn assert_send<T: Send>() {}

    #[test]
    fn pending_is_send() {
        assert_send::<Pending<'static>>();
    }
}

#[cfg(all(test, feature = "cookie"))]
mod cookie_tests {
    use std::time::Duration;

    use http::{HeaderMap, Request, header};
    use topcoat_cookie::{Cookie, CookieJarCell, Cookies, cookies, write_cookies};

    use super::*;
    use crate::{
        SessionLifetime, SessionTokenStore, TokenStoreFuture,
        cookie::{CookieTokenStore, SESSION_COOKIE_NAME},
    };

    /// Builds a request context with a cookie jar and an optional `Cookie`
    /// header.
    fn cx_with(cookie_header: Option<&str>) -> Cx {
        let mut builder = Request::builder();
        if let Some(value) = cookie_header {
            builder = builder.header(header::COOKIE, value);
        }
        let (parts, ()) = builder.body(()).unwrap().into_parts();
        Cx::new().with_many((parts, CookieJarCell::new()))
    }

    fn cx_without_session() -> Cx {
        cx_with(None)
    }

    fn session_cookie(token: &Token) -> String {
        format!("__Host-{SESSION_COOKIE_NAME}={}", token.encode())
    }

    /// A context whose request carries `token` in the default session cookie.
    fn cx_with_session(token: &Token) -> Cx {
        cx_with(Some(&session_cookie(token)))
    }

    /// The `Set-Cookie` values the response would carry.
    fn set_cookies(cx: &Cx) -> Vec<String> {
        let mut headers = HeaderMap::new();
        write_cookies(cx, &mut headers);
        headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect()
    }

    /// The session token issued by a `Set-Cookie` value, or `None` for a
    /// removal or a non-session cookie.
    fn issued_token(set_cookie: &str) -> Option<Token> {
        let cookie = Cookie::parse_encoded(set_cookie).unwrap();
        if cookie.name() != format!("__Host-{SESSION_COOKIE_NAME}") {
            return None;
        }
        Token::decode(cookie.value()).ok()
    }

    fn issued_tokens(cx: &Cx) -> Vec<Token> {
        set_cookies(cx)
            .iter()
            .filter_map(|cookie| issued_token(cookie))
            .collect()
    }

    /// The `Max-Age` a `Set-Cookie` value carries.
    fn max_age(set_cookie: &str) -> Duration {
        let max_age = Cookie::parse_encoded(set_cookie)
            .unwrap()
            .max_age()
            .unwrap();
        Duration::try_from(max_age).unwrap()
    }

    /// The error a [`FailingStore`] write returns.
    #[derive(Debug, thiserror::Error)]
    #[error("store down")]
    struct StoreDown;

    /// Reads like the cookie store but refuses every write.
    struct FailingStore(CookieTokenStore);

    impl TokenStore for FailingStore {
        fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
            self.0.read(cx)
        }

        fn write<'a>(
            &'a self,
            _cx: &'a Cx,
            _token: Token,
            _time_to_live: Duration,
        ) -> TokenStoreFuture<'a, ()> {
            Box::pin(async { Err(Error::from(StoreDown)) })
        }

        fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
            self.0.delete(cx)
        }
    }

    #[tokio::test]
    async fn dropping_a_pending_issues_nothing() {
        let cx = cx_without_session();
        let pending = prepare_start(&cx);
        drop(pending);
        assert_eq!(set_cookies(&cx), Vec::<String>::new());
    }

    #[tokio::test]
    async fn the_request_view_changes_only_on_commit() {
        let arriving = Token::random();
        let cx = cx_with_session(&arriving);
        let pending = prepare_start(&cx);
        let prepared = pending.session().clone();
        assert_eq!(token_hash(&cx).await.unwrap(), Some(arriving.hash()));
        let committed = pending.commit().await.unwrap();
        assert_eq!(committed, prepared);
        assert_eq!(
            token_hash(&cx).await.unwrap(),
            Some(committed.token_hash.clone())
        );
        assert_ne!(committed.token_hash, arriving.hash());
    }

    #[tokio::test]
    async fn commit_issues_the_prepared_token() {
        for cx in [cx_without_session(), cx_with_session(&Token::random())] {
            let committed = prepare_start(&cx).commit().await.unwrap();
            let issued = issued_tokens(&cx);
            assert_eq!(issued.len(), 1);
            assert_eq!(issued[0].hash(), committed.token_hash);
        }
    }

    #[tokio::test]
    async fn a_failed_commit_returns_the_unissued_session() {
        let store = SessionTokenStore::new(FailingStore(CookieTokenStore::new()));
        let arriving = Token::random();
        let cx = cx_with_session(&arriving).with(store);
        let pending = prepare_start(&cx);
        let prepared = pending.session().clone();
        let error = pending.commit().await.unwrap_err();
        assert_eq!(*error.session(), prepared);
        let (session, source) = error.into_parts();
        assert_eq!(session, prepared);
        assert!(source.is::<StoreDown>());
        assert_eq!(token_hash(&cx).await.unwrap(), Some(arriving.hash()));
        assert_eq!(set_cookies(&cx), Vec::<String>::new());
    }

    #[tokio::test]
    async fn start_and_rotate_return_the_store_error_unchanged() {
        let store = SessionTokenStore::new(FailingStore(CookieTokenStore::new()));
        let cx = cx_with_session(&Token::random()).with(store);
        assert!(start(&cx).await.unwrap_err().is::<StoreDown>());
        assert!(rotate(&cx).await.unwrap_err().is::<StoreDown>());
    }

    #[tokio::test]
    async fn the_record_expires_no_later_than_the_cookie() {
        let cx = cx_without_session().with(SessionLifetime(Duration::from_millis(1900)));
        let pending = prepare_start(&cx);
        let expires_at = pending.session().expires_at;
        let committed_at = SystemTime::now();
        pending.commit().await.unwrap();
        let cookie_expires_at = committed_at + max_age(&set_cookies(&cx)[0]);
        assert!(expires_at <= cookie_expires_at);
    }

    #[tokio::test]
    async fn a_lifetime_beyond_max_age_is_an_error_not_a_panic() {
        let cx = cx_without_session();
        let result = CookieTokenStore::new()
            .write(&cx, Token::random(), Duration::MAX)
            .await;
        assert!(result.is_err());
        assert_eq!(set_cookies(&cx), Vec::<String>::new());
    }

    #[tokio::test]
    async fn prepare_rotate_without_a_session_is_none() {
        let cx = cx_without_session();
        assert!(prepare_rotate(&cx).await.unwrap().is_none());
        assert_eq!(set_cookies(&cx), Vec::<String>::new());
    }

    #[tokio::test]
    async fn prepare_rotate_with_a_malformed_cookie_is_none() {
        let cx = cx_with(Some(&format!("__Host-{SESSION_COOKIE_NAME}=not-a-token")));
        assert!(prepare_rotate(&cx).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn a_committed_rotation_matches_rotate() {
        let arriving = Token::random();
        let prepared_cx = cx_with_session(&arriving);
        let rotation = prepare_rotate(&prepared_cx).await.unwrap().unwrap();
        assert_eq!(rotation.revoked, arriving.hash());
        let committed = rotation.pending.commit().await.unwrap();

        let rotated_cx = cx_with_session(&arriving);
        let direct = rotate(&rotated_cx).await.unwrap().unwrap();
        assert_eq!(direct.revoked, rotation.revoked);

        let prepared_cookies = set_cookies(&prepared_cx);
        let rotated_cookies = set_cookies(&rotated_cx);
        assert_eq!(prepared_cookies.len(), 1);
        assert_eq!(rotated_cookies.len(), 1);
        assert_eq!(
            issued_token(&prepared_cookies[0]).unwrap().hash(),
            committed.token_hash
        );
        let attributes = |cookie: &str| cookie.split_once(';').map(|(_, rest)| rest.to_owned());
        assert_eq!(
            attributes(&prepared_cookies[0]),
            attributes(&rotated_cookies[0])
        );
    }

    #[tokio::test]
    async fn other_cookies_survive_a_dropped_pending() {
        let cx = cx_with_session(&Token::random());
        cookies(&cx).add(Cookie::new("theme", "dark"));
        let _ = prepare_rotate(&cx).await.unwrap();
        let emitted = set_cookies(&cx);
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].starts_with("theme=dark"));
    }

    #[tokio::test]
    async fn revoked_follows_the_request_view() {
        let cx = cx_with_session(&Token::random());
        let first = prepare_rotate(&cx).await.unwrap().unwrap();
        let committed = first.pending.commit().await.unwrap();
        let second = prepare_rotate(&cx).await.unwrap().unwrap();
        assert_eq!(second.revoked, committed.token_hash);
        drop(second);
        stop(&cx).await.unwrap();
        assert!(prepare_rotate(&cx).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn commit_after_stop_issues_the_token() {
        let cx = cx_with_session(&Token::random());
        let rotation = prepare_rotate(&cx).await.unwrap().unwrap();
        stop(&cx).await.unwrap();
        let committed = rotation.pending.commit().await.unwrap();
        assert_eq!(
            token_hash(&cx).await.unwrap(),
            Some(committed.token_hash.clone())
        );
        let issued = issued_tokens(&cx);
        assert_eq!(issued.len(), 1);
        assert_eq!(issued[0].hash(), committed.token_hash);
    }

    #[tokio::test]
    async fn a_pending_dropped_after_stop_keeps_the_removal() {
        let cx = cx_with_session(&Token::random());
        let rotation = prepare_rotate(&cx).await.unwrap().unwrap();
        stop(&cx).await.unwrap();
        drop(rotation);
        assert_eq!(token_hash(&cx).await.unwrap(), None);
        let emitted = set_cookies(&cx);
        assert_eq!(emitted.len(), 1);
        assert!(issued_token(&emitted[0]).is_none());
        assert!(emitted[0].starts_with(&format!("__Host-{SESSION_COOKIE_NAME}=")));
    }
}
