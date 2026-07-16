Topcoat sessions implement the *mechanics* of session authentication -- generating tokens, carrying them between client and server, and the login/logout lifecycle -- while **you own the storage**. The framework hands you a hash and an expiry to persist in your own database, with your own ORM and schema; it never dictates a session table or a user model. The API is deliberately minimal for now and will likely be expanded over time.

Sessions are part of the default feature set, and everything below is re-exported from `topcoat::session`.

# The model

A session is identified by a **token**: 32 bytes of cryptographically secure randomness that only the client holds. By default the token travels in a hardened session cookie (`__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, scoped to `/`).

Your application never stores the raw token. It persists the token's SHA-256 hash, a [`TokenHash`], next to whatever the session authenticates (typically a user id) and the session's expiry. Because the hash cannot be turned back into a token, a leaked session database contains nothing a client could present.

The lifecycle is a handful of functions taking `cx: &Cx`:

- [`start`] mints a fresh token, issues it to the client, and returns the [`Session`] (hash and expiry) for you to record. Call it on login.
- [`token_hash`] returns the hash of the token the current request presented, for you to look up in your storage.
- [`stop`] instructs the client to discard its token and returns the hash so you can delete the record. Call it on logout.
- [`refresh`] re-issues the current token with a full lifetime ahead of it, for sliding expiration.
- [`rotate`] replaces the current token with a fresh one, for privilege changes.

Within a request the presented token is read once and cached, and [`start`], [`stop`], and [`rotate`] update that cached view, so a page rendered after a login sees the new session immediately.

# Setup

Register session support on the router with [`RouterBuilderSessionExt::sessions`]. The default [`Config`] carries the token in a session cookie, which needs cookie support installed as well:

```rust
use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::Router,
    session::{Config, RouterBuilderSessionExt},
};

let router = Router::builder()
    .cookies()
    .sessions(Config::default())
    .build();
```

# Logging in

Authenticate the user however your application does, then call [`start`] and record the returned [`Session`] in your storage. [`start`] always generates a fresh token -- it never reuses one the request presented -- so it also protects against session fixation.

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{SeeOther, route, see_other},
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

[`token_hash`] gives you the hash for the request's token, or `None` when the request carries no (valid) token. Looking it up is your side of the contract, and the idiomatic shape is a `current_user` function in the spirit of [functions, not middlewares](functions_not_middlewares.md). Treat a hash your storage does not contain, or whose record has expired, as not authenticated:

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

The token itself is only read once per request, but `current_user`'s database lookup runs on every call; wrap it with [`#[memoize]`](memoization.md) if pages call it repeatedly.

Guard pages by combining it with the router's error helpers:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{RouterErrorExt, page},
    view::view,
};
# #[derive(Clone)] struct User { name: String }
# async fn current_user(_cx: &Cx) -> Result<Option<User>> { Ok(None) }

#[page("/account")]
async fn account(cx: &Cx) -> Result {
    let user = current_user(cx).await?.ok_or_redirect("/login")?;
    view! {
        <h1>"Account of " (&user.name)</h1>
    }
}
```

# Logging out

[`stop`] tells the client to discard its token and hands back the hash of the session it ended, so you can delete the record:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{SeeOther, route, see_other},
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

Note that [`stop`] only ends the session the request presented. Revoking *other* sessions (a "sign out everywhere" button) is a matter of deleting their records from your storage; their tokens stop resolving the moment the records are gone.

# Refreshing and rotating

A session expires a fixed [`lifetime`](ConfigBuilder::lifetime) after it was started. For **sliding expiration** -- sessions that stay alive while they are used -- call [`refresh`] when you resolve a valid session and push the expiry of your record forward:

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

[`rotate`] keeps the session but swaps its token for a fresh one, so a token that leaked before the rotation stops working. Rotate when a session's privilege changes (for example after re-authenticating for a sensitive action). It returns a [`Rotation`]: revoke the record under `rotation.revoked` and record `rotation.session` in its place.

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

[`Config`] holds the token store and the session lifetime (30 days unless overridden), and is assembled with [`Config::builder`]. The default cookie store can be renamed if the `session` cookie name does not suit:

```rust
use std::time::Duration;

use topcoat::session::{Config, cookie::CookieTokenStore};

let config = Config::builder()
    .token_store(CookieTokenStore::new().name("id"))
    .lifetime(Duration::from_hours(24 * 14))
    .build();
```

The lifetime becomes both the `Max-Age` of the issued cookie and the `expires_at` handed to you by [`start`], [`refresh`], and [`rotate`], so the client's cookie and your record expire together.

# Cross-origin request protection

`.sessions()` also registers the [`OriginLayer`], which rejects state-changing cross-origin requests with `403 Forbidden` before they reach a route. This closes the cross-site request forgery (CSRF) gaps that the session cookie's `SameSite=Lax` leaves open, such as requests forged from a sibling subdomain.

For every request whose method is not `GET`, `HEAD`, or `OPTIONS`, the layer requires the browser-provided `Sec-Fetch-Site` header to be `same-origin` or `none` (a direct navigation). For older browsers that do not send it, the `Origin` header's host is compared against the request's own host instead. Requests carrying neither header pass: they come from non-browser clients like `curl` or server-to-server calls, which do not attach cookies ambiently and so cannot be forged through a victim's browser.

If a page on another origin legitimately POSTs to your app (an OAuth `form_post` callback, for example), trust that origin explicitly:

```rust
use topcoat::session::Config;

let config = Config::builder()
    .trust_origin("https://accounts.example.com")
    .build();
```

The check is also available as a plain function, [`verify_origin`], for flows outside the layer. [`ConfigBuilder::dangerous_disable_origin_verification`] turns the layer off entirely; only do so if the application enforces its own CSRF defense.

# Custom token stores

A [`TokenStore`] is the client-side transport for the token; it is *not* the session database. Implement it to carry the token somewhere other than the default cookie, for example an `Authorization` header for API clients:

```rust
use std::time::Duration;

use topcoat::{
    context::Cx,
    router::headers,
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

- The default cookie is as locked down as a session cookie can be: `__Host-` prefixed, `Secure`, `HttpOnly`, `SameSite=Lax`, and scoped to `/`. It is invisible to scripts and never sent cross-site on subresource or scripted requests.
- `SameSite=Lax` still sends the cookie on top-level cross-site navigations, so keep every state-changing route on `POST` (or another non-`GET` method), as the examples above do. The [`OriginLayer`] then rejects any such request that does arrive cross-origin; safe methods are deliberately not checked, so a state-changing `GET` remains unprotected.
- Compare sessions by looking the hash up in your storage; never store or log the raw token server-side.
