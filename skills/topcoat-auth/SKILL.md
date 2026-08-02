---
name: topcoat-auth
description: Implement, review, test, and debug authentication and authorization in Topcoat Rust applications. Use for cookies, signed or private jars, typed cookie stores, sessions, login and logout, session storage, current-user helpers, access guards, token rotation, sliding expiration, CSRF and origin checks, and protecting procedures or shards.
---

# Topcoat Auth

Keep authorization close to the data it protects. Prefer composable `cx: &Cx` functions over hidden middleware, and check the current cookie and session guides before relying on detailed APIs.

## Configure cookies and sessions

```rust
use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::Router,
    session::{RouterBuilderSessionExt, SessionConfig},
};

let router = Router::builder()
    .cookies()
    .sessions(SessionConfig::default())
    .app_context(Database::connect().await?)
    .build();
```

Topcoat owns token generation, client transport, and lifecycle; the application owns session records. Store only `Session.token_hash`, the authenticated subject, and `expires_at`. Never store or log the raw token.

## Implement the lifecycle

- Login: Verify credentials, call `session::start(cx)`, and persist the returned session.
- Current user: Read `session::token_hash(cx)`, load an unexpired record, and return the subject.
- Logout: Call `session::stop(cx)` and delete the returned hash.
- Sliding expiry: Call `session::refresh(cx)` and update the stored expiry.
- Privilege change: Call `session::rotate(cx)` and atomically replace the revoked hash with the new session.
- Sign out everywhere: Delete all relevant records in application storage.

Memoize the current-user lookup so layouts, pages, and nested components share one database query:

```rust
use topcoat::{Result, context::{Cx, memoize}, router::error::RouterErrorExt};

#[memoize]
async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = topcoat::session::token_hash(cx).await? else {
        return Ok(None);
    };
    Ok(db(cx).find_unexpired_session(&hash).await?)
}

async fn require_auth(cx: &Cx) -> Result<&User> {
    Ok(current_user(cx).await?.ok_or_unauthorized()?)
}
```

Build `require_admin`, tenant membership, ownership, and similar checks from focused helpers. Call the appropriate guard inside every page, component, API route, procedure, and shard that exposes protected data. Procedures and shards have separate endpoints and do not inherit page or layout checks.

## Choose cookie protection correctly

- Plain cookie: Client-readable and forgeable; use only for non-sensitive preferences.
- Signed jar: Client-readable but tamper-evident.
- Private jar: Encrypted and authenticated for sensitive values.
- `CookieStore<T>`: Structured JSON state over any jar; changes are not written until `commit()`.

Generate signing keys once, store them durably outside source control, and register them as app context. Regenerating a key on boot invalidates existing signed and private cookies. Reuse the same cookie name, path, domain, and prefix when removing it.

Prefer the default hardened session cookie. Keep state-changing operations on non-safe methods such as `POST`; never mutate state through `GET`.

## Preserve origin protection

`.sessions()` installs origin verification for state-changing browser requests. Trust a specific external origin only for a legitimate flow such as an OAuth form-post callback. Disable verification only when another complete CSRF defense is in place.

Test missing, malformed, expired, revoked, and rotated sessions; cookie tampering; login fixation resistance; authorization at direct procedure and shard URLs; origin rejection; and failure paths around session storage updates.
