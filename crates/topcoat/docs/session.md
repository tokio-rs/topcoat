Topcoat manages session tokens. Your application stores each token's hash and expiry, associates it with a user, and checks that record on later requests. You choose the database and schema.

Sessions are part of the default feature set, and everything below is re-exported from `topcoat::session`.

# The model

A session token contains 32 bytes of cryptographically secure randomness. The client sends it with each request. By default, Topcoat carries it in a cookie with the `__Host-` prefix, `Secure`, `HttpOnly`, `SameSite=Lax`, and `Path=/`.

Store the token's SHA-256 hash, a [`TokenHash`], alongside the user ID and expiry. Do not store the raw token. Someone who obtains the session records cannot use a hash as a session token.

Session functions take `cx: &Cx`:

- [`start`] mints a fresh token, issues it to the client, and returns the [`Session`] (hash and expiry) for you to record. Call it on login.
- [`token_hash`] returns the hash of the token the current request presented, for you to look up in your storage.
- [`stop`] instructs the client to discard its token and returns the hash so you can delete the record. Call it on logout.
- [`refresh`] re-issues the current token with a full lifetime ahead of it, for sliding expiration.
- [`rotate`] replaces the current token with a fresh one, for privilege changes.

Topcoat reads the token once per request. Session changes update that cached value, so later code in the same request sees the new session.

Changing the session involves setting cookies, which is only possible if the response body has not begun streaming yet. The [cookie guide](crate::cookie#writes-must-happen-before-the-response) explains this in more detail.

# Setup

Register session support on the router with [`RouterBuilderSessionExt::sessions`]. The default [`SessionConfig`] carries the token in a session cookie, which needs cookie support installed as well:

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

After authenticating the user, call [`start`] and store the returned [`Session`]. It always generates a fresh token. This prevents session fixation, where an attacker tries to make a user log in with a token the attacker already knows.

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

`Session.token_hash` is the key of the record, and `Session.expires_at` is when it stops being valid. Persist both.

# Resolving the current user

[`token_hash`] returns the request token's hash, or `None` if the token is missing or malformed. Look up the hash in your storage and reject missing or expired records. A `current_user` helper keeps this check available wherever it is needed, as described in [functions, not middlewares](crate::context#functions-not-middlewares):

```rust
use topcoat::{Result, context::Cx, session};
# #[derive(Clone)] struct User;
# async fn load_session_user(_cx: &Cx, _hash: &session::TokenHash) -> Result<Option<User>> { Ok(None) }

async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };
    // Your storage: return the user only while the record is unexpired.
    load_session_user(cx, &hash).await
}
```

The database lookup runs on every call to `current_user`. Use [`#[memoize]`](macro@crate::context::memoize) if several callers need the result during one request.

Guard pages by combining it with the router's error helpers:

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

[`stop`] tells the client to discard its token and hands back the hash of the session it ended, so you can delete the record:

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

[`stop`] only clears the current session. To revoke other sessions, delete their records from your storage. Later lookups must reject those tokens.

# Refreshing and rotating

A session expires after its configured [`lifetime`](SessionConfigBuilder::lifetime). To keep an active session alive, call [`refresh`] after validating it and save the new expiry:

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

[`rotate`] issues a fresh token and returns a [`Rotation`]. Delete the record identified by `rotation.revoked` and save `rotation.session` in its place. Once you delete the old record, the old token no longer authenticates. Rotate after a change in privileges, such as re-authentication for a sensitive action.

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

Use [`SessionConfig::builder`] to choose a token store and lifetime. For example, this configuration changes the cookie name and sets a two-week lifetime:

```rust
use std::time::Duration;

use topcoat::session::{SessionConfig, cookie::CookieTokenStore};

let config = SessionConfig::builder()
    .token_store(CookieTokenStore::new().name("id"))
    .lifetime(Duration::from_hours(24 * 14))
    .build();
```

The lifetime becomes both the `Max-Age` of the issued cookie and the `expires_at` handed to you by [`start`], [`refresh`], and [`rotate`], so the client's cookie and your record expire together.

# Custom token stores

A [`TokenStore`] reads tokens from requests and sends token changes to the client. Session records remain in your application's storage. Implement this trait to use another transport, such as an `Authorization` header:

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
        // API clients receive their token out of band; there is nothing to
        // send with the response.
        Box::pin(async move { Ok(()) })
    }

    fn delete<'a>(&'a self, _cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
```

Serialize the raw token with [`Token::encode`] and parse it back with [`Token::decode`]; both use URL-safe base64.

# Security notes

- The default cookie requires HTTPS and cannot be read by client-side scripts. Its `__Host-` prefix restricts it to the issuing host.
- `SameSite=Lax` permits cookies on top-level cross-site navigations. Use `POST` or another unsafe HTTP method for state changes. The router's default [`OriginPolicy`](crate::router::OriginPolicy) checks these requests, but does not protect a state-changing `GET`.
- Compare sessions by looking the hash up in your storage; never store or log the raw token server-side.
