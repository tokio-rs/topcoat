Topcoat creates session tokens and sends them to clients. Your application stores each token's hash and expiry, associates it with a user, and checks that record when authenticating a request.

Sessions are part of the default feature set, and everything below is re-exported from `topcoat::session`.

# The model

A session is identified by a **token**: 32 bytes of cryptographically secure randomness that only the client holds. By default the token travels in a hardened session cookie (`__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, scoped to `/`).

Store the token's [`TokenHash`] and expiry with the user it authenticates. Do not store the raw token. The hash is a lookup key, not a credential a client can present.

Session changes are visible to later reads in the same request.

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

After checking the user's credentials, call [`start`] and store the returned [`Session`]. It always issues a fresh token, which protects against session fixation.

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

[`token_hash`] returns the presented token's hash, or `None` if the token is absent or malformed. Look up the hash in your storage and check the record's expiry before accepting it:

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

The token itself is only read once per request, but `current_user`'s database lookup runs on every call; wrap it with [`#[memoize]`](macro@crate::context::memoize) if pages call it repeatedly.

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

To revoke other sessions, delete their records from your storage. Their tokens will no longer resolve to an authenticated user.

# Refreshing and rotating

A session expires a fixed [`lifetime`](SessionConfigBuilder::lifetime) after it was started. For **sliding expiration** -- sessions that stay alive while they are used -- call [`refresh`] when you resolve a valid session and push the expiry of your record forward:

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

[`rotate`] issues a fresh token. Use it after a privilege change, then revoke the record under `rotation.revoked` and store `rotation.session`. The old token stops authenticating requests only after you revoke its record.

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

Use [`SessionConfig::builder`] to configure the lifetime and how tokens reach the client. For example, change the cookie name and lifetime:

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

A [`TokenStore`] is the client-side transport for the token; it is *not* the session database. Implement it to carry the token somewhere other than the default cookie, for example an `Authorization` header for API clients:

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

- The default cookie uses a `__Host-` prefix, `Secure`, `HttpOnly`, `SameSite=Lax`, and `Path=/`. Serve the application over HTTPS.
- `SameSite=Lax` still sends the cookie on top-level cross-site navigations, so keep every state-changing route on `POST` (or another non-`GET` method), as the examples above do. The router's [`OriginPolicy`](crate::router::OriginPolicy) then rejects any such request that does arrive cross-origin; safe methods are deliberately not checked, so a state-changing `GET` remains unprotected.
- Compare sessions by looking the hash up in your storage; never store or log the raw token server-side.
