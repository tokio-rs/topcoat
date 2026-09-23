Topcoat sessions handle the mechanics of session authentication: they generate tokens, carry them between the client and the server, and provide the login and logout lifecycle. You own the storage. Topcoat gives you a hash and an expiry time to save in your own database, with your own schema. It does not require a particular session table or user model.

Sessions are part of the default feature set, and everything below lives in `topcoat::session`.

# The model

A session is identified by a token: 32 random bytes from a cryptographically secure source, which only the client holds. By default the token travels in a session cookie that is `__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, and scoped to `/`.

Your application never stores the token itself. It stores the token's SHA-256 hash, a [`TokenHash`], together with whatever the session authenticates (usually a user id) and the session's expiry time. The token cannot be recovered from the hash, so a leaked session database contains nothing a client could use to log in.

The lifecycle consists of a few functions that take `cx: &Cx`:

- [`start`] creates a new token, sends it to the client, and returns the [`Session`] (hash and expiry time) for you to save. Call it on login.
- [`token_hash`] returns the hash of the token the current request carries, for you to look up in your storage.
- [`stop`] tells the client to discard its token and returns the hash, so you can delete the record. Call it on logout.
- [`refresh`] sends the current token again with a full lifetime, for sliding expiration.
- [`rotate`] replaces the current token with a new one, for example after a privilege change.

The request's token is read at most once per request. [`start`], [`stop`], and [`rotate`] update the request's view of the token, so code that runs later in the same request, such as the page rendered after a login, already sees the new session.

With the default cookie store, changing the session sets a cookie, which is only possible before the response headers are sent. The [cookie guide](crate::cookie#writes-must-happen-before-the-response) explains this in more detail.

# Setup

Add session support to the router with [`RouterBuilderSessionExt::sessions`]. The default [`SessionConfig`] carries the token in a cookie, so it also needs cookie support:

```rust
use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::Router,
    session::{RouterBuilderSessionExt, SessionConfig},
};

let router = Router::builder()
    .cookies()
    .sessions(SessionConfig::default())
    .build();
```

# Logging in

Check the user's credentials the way your application does, then call [`start`] and save the returned [`Session`] in your storage. [`start`] always creates a new token and never reuses the one the request carried, which protects against session fixation.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{error::{SeeOther, see_other}, route},
    session,
};
# struct User;
# async fn verify_credentials(_cx: &Cx) -> Result<User> { Ok(User) }
# async fn persist_session(_cx: &Cx, _user: &User, _session: &session::Session) -> Result<()> { Ok(()) }

#[route(POST "/login")]
async fn login(cx: &Cx) -> Result<SeeOther> {
    let user = verify_credentials(cx).await?;

    let session = session::start(cx).await?;
    persist_session(cx, &user, &session).await?;

    Ok(see_other("/"))
}
```

Save both fields of the [`Session`]: `token_hash` identifies the record, and `expires_at` is when the session stops being valid.

# Resolving the current user

[`token_hash`] returns the hash of the request's token, or `None` when the request carries no valid token. Looking up the hash is up to you. A good pattern is a `current_user` function, as described in [functions, not middlewares](crate::context#functions-not-middlewares). Treat a hash that your storage does not contain, or whose record has expired, as not logged in:

```rust
use topcoat::{Result, context::Cx, session};
# #[derive(Clone)] struct User;
# async fn load_session_user(_cx: &Cx, _hash: &session::TokenHash) -> Result<Option<User>> { Ok(None) }

async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };
    // Your storage: return the user only while the record has not expired.
    load_session_user(cx, &hash).await
}
```

The token is read only once per request, but the database lookup in `current_user` runs on every call. Add [`#[memoize]`](macro@crate::context::memoize) to it if a request calls it more than once.

To protect a page, combine it with the router's error helpers:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, page},
    view::{View, view},
};
# #[derive(Clone)] struct User { name: String }
# async fn current_user(_cx: &Cx) -> Result<Option<User>> { Ok(None) }

#[page("/account")]
async fn account(cx: &Cx) -> Result<impl View> {
    let user = current_user(cx).await?.ok_or_redirect("/login")?;
    Ok(view! {
        <h1>"Account of " (&user.name)</h1>
    })
}
```

# Logging out

[`stop`] tells the client to discard its token and returns the hash of the session it ended, so you can delete the record:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{error::{SeeOther, see_other}, route},
    session,
};
# async fn delete_session(_cx: &Cx, _hash: &session::TokenHash) -> Result<()> { Ok(()) }

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    if let Some(hash) = session::stop(cx).await? {
        delete_session(cx, &hash).await?;
    }
    Ok(see_other("/"))
}
```

[`stop`] only ends the session of the current request. To end other sessions, for example with a "sign out everywhere" button, delete their records from your storage. Their tokens stop working as soon as the records are gone.

# Refreshing and rotating

A session expires a fixed [`lifetime`](SessionConfigBuilder::lifetime) after it starts. For sliding expiration, where a session stays valid as long as it is used, call [`refresh`] when you find a valid session and update the expiry time of your record:

```rust
use topcoat::{Result, context::Cx, session};
# use std::time::SystemTime;
# async fn update_session_expiry(_cx: &Cx, _hash: &session::TokenHash, _expires_at: SystemTime) -> Result<()> { Ok(()) }

async fn slide_expiration(cx: &Cx) -> Result<()> {
    if let Some(session) = session::refresh(cx).await? {
        update_session_expiry(cx, &session.token_hash, session.expires_at).await?;
    }
    Ok(())
}
```

[`rotate`] keeps the session but replaces its token with a new one, so a token that leaked before the rotation stops working. Rotate when the privileges of a session change, for example after the user confirms their password for a sensitive action. It returns a [`Rotation`]: delete the record under `rotation.revoked` and save `rotation.session` instead, or move the record to the new hash.

```rust
use topcoat::{Result, context::Cx, session};
# async fn rekey_session(_cx: &Cx, _revoked: &session::TokenHash, _session: &session::Session) -> Result<()> { Ok(()) }

async fn escalate(cx: &Cx) -> Result<()> {
    if let Some(rotation) = session::rotate(cx).await? {
        rekey_session(cx, &rotation.revoked, &rotation.session).await?;
    }
    Ok(())
}
```

# Configuration

A [`SessionConfig`] holds the token store and the session lifetime, which is 30 days by default ([`DEFAULT_LIFETIME`]). Build one with [`SessionConfig::builder`]. For example, to rename the cookie of the default cookie store and shorten the lifetime:

```rust
use std::time::Duration;

use topcoat::session::{SessionConfig, cookie::CookieTokenStore};

let config = SessionConfig::builder()
    .token_store(CookieTokenStore::new().name("id"))
    .lifetime(Duration::from_hours(24 * 14))
    .build();
```

The lifetime sets both the `Max-Age` of the session cookie and the `expires_at` returned by [`start`], [`refresh`], and [`rotate`], so the cookie and your record expire together.

# Custom token stores

A [`TokenStore`] carries the token between the client and the server. It is not the session database. Implement it to carry the token somewhere other than a cookie, for example in an `Authorization` header for API clients:

```rust
use std::time::Duration;

use topcoat::{
    context::Cx,
    router::request::headers,
    session::{Token, TokenStore, TokenStoreFuture},
};

struct BearerTokenStore;

impl TokenStore for BearerTokenStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            let Some(bearer) = headers(cx)
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
            else {
                return Ok(None);
            };
            Ok(Token::decode(bearer).ok())
        })
    }

    fn write<'a>(
        &'a self,
        _cx: &'a Cx,
        _token: Token,
        _max_age: Duration,
    ) -> TokenStoreFuture<'a, ()> {
        // API clients receive their token some other way, so there is
        // nothing to send with the response.
        Box::pin(async move { Ok(()) })
    }

    fn delete<'a>(&'a self, _cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
```

Use [`Token::encode`] to turn a token into text and [`Token::decode`] to parse it back. Both use URL-safe base64.

# Security notes

- The default session cookie is `__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, and scoped to `/`. Scripts cannot read it, and browsers do not send it with cross-site subresource or script requests.
- `SameSite=Lax` still sends the cookie when a user follows a link from another site. Keep every route that changes state on `POST` or another non-`GET` method, as the examples above do. The router's [`OriginPolicy`](crate::router::OriginPolicy) rejects such requests when they come from another origin. It does not check `GET` and other safe methods, so a `GET` route that changes state is not protected.
- Look sessions up by their hash in your storage. Never store or log the token itself on the server.
