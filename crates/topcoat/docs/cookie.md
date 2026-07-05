Topcoat reads and writes cookies through a request-scoped **cookie jar**. Install cookie support on your router with `.cookies()`, then call `cookies(cx)` from any handler to get the jar, read incoming cookies, and queue changes. Anything you add or remove during the request is serialized into `Set-Cookie` response headers automatically once the handler returns. You don't need to touch headers yourself.

Cookies are part of the default feature set, and everything below is re-exported from `topcoat::cookie`. Topcoat builds on the `cookie` crate: a cookie is a [`Cookie`], and signing and encryption use its [`Key`].

```rust
use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::Router,
};

let router = Router::builder()
    .cookies()
    .build();
```

# Reading and writing

`cookies(cx)` returns the request's root jar. The incoming `Cookie` header is parsed on first access and memoized for the rest of the request, so repeated calls are cheap and see the same pending changes. Bring the [`Cookies`] trait into scope for the `get`, `add`, and `remove` methods.

A cookie is a [`Cookie`] from the `cookie` crate. Build a bare one with `Cookie::new`, or use `Cookie::build` for attributes:

```rust
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookie, Cookies, cookies},
    router::route,
};

#[route(POST "/api/theme")]
async fn toggle_theme(cx: &Cx) -> Result<String> {
    let jar = cookies(cx);

    let next = match jar.get("theme") {
        Some(theme) if theme.value() == "dark" => "light",
        _ => "dark",
    };

    jar.add(Cookie::build(("theme", next)).path("/").build());

    Ok(next.to_owned())
}
```

- `get(name)` returns the cookie if the request carried it (or `None`).
- `add(cookie)` queues a `Set-Cookie`.
- `remove(cookie)` queues an expiring removal cookie. Pass the same `Path`/`Domain` the cookie was set with so the browser matches and clears it:

```rust
# use topcoat::cookie::{Cookie, Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
# let jar = cookies(cx);
jar.remove(Cookie::build(("session", "")).path("/").build());
# }
```

`add` and `remove` accept anything that implements `Into<Cookie>`, so a plain `(name, value)` tuple works too when you don't need attributes:

```rust
# use topcoat::cookie::{Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
# let jar = cookies(cx);
jar.add(("theme", "dark"));
# }
```

# Building cookies with `cookie!`

For cookies with several attributes, the [`cookie!`] macro is more compact than the builder. It mirrors the [`Set-Cookie`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie) header: the `name = value` pair first, then any number of `;`-separated attributes.

```rust
use topcoat::cookie::{Cookie, SameSite, cookie, time::Duration};

let plain: Cookie = cookie!("theme" = "dark");

let session: Cookie = cookie! {
    "session" = "abc123";
    Path = "/";
    Secure;
    HttpOnly;
    SameSite = Lax;
    MaxAge = Duration::hours(1)
};
```

# Default and override attributes

Rather than repeat the same attributes on every cookie, layer them onto the jar with the [`Cookies`] combinators. Each one wraps the jar and applies an attribute to cookies added through it, in the style of [`Iterator`] adapters. Every attribute comes in two flavors:

- `default_*` fills the attribute only when the cookie does not already set it.
- `override_*` forces the attribute, replacing any value the cookie had.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, SameSite, cookie, cookies};

let jar = cookies(cx)
    .default_secure(true)
    .default_http_only(true)
    .default_same_site(SameSite::Lax)
    .default_path("/");

// Picks up Secure, HttpOnly, SameSite=Lax, Path=/ from the defaults above.
jar.add(cookie!("session" = "abc123"));
# }
```

The same pairs exist for `path`, `domain`, and `max_age`. Because the combinators consume and return the jar, build the configured jar once and reuse it for several writes.

Rather than repeat that chain at every call site, the idiomatic pattern is to write your own `cookies` helper that bakes in your app's defaults and have the rest of your code use it instead of the one from `topcoat::cookie`:

```rust
use topcoat::{
    context::Cx,
    cookie::{Cookies, SameSite},
};

/// The application cookie jar, with our security defaults applied.
fn cookies(cx: &Cx) -> impl Cookies {
    topcoat::cookie::cookies(cx)
        .default_secure(true)
        .default_http_only(true)
        .default_same_site(SameSite::Lax)
        .default_path("/")
}
```

Every handler that calls this `cookies(cx)` gets the defaults for free, and you can tighten them in one place. The same approach works for signed or private jars: return `impl Cookies` and layer on whatever combinators your app needs.

For anything the named combinators don't cover, [`map`](Cookies::map) is the escape hatch; it runs a closure on every added cookie:

```rust
# use topcoat::cookie::{Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
let jar = cookies(cx).map(|cookie| cookie.set_partitioned(true));
# }
```

# Name prefixes

[RFC 6265bis] cookie name prefixes ask the browser to enforce extra constraints based on the cookie's name. Topcoat applies the prefix *and* its required attributes for you, and strips the prefix back off on read so your code keeps using the bare name.

- `prefix_host` (`__Host-`): the cookie must be `Secure`, have `Path=/`, and carry no `Domain`. The tightest scoping: bound to the exact host, unavailable to subdomains.
- `prefix_secure` (`__Secure-`): the cookie must be `Secure`.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, cookie, cookies};

let jar = cookies(cx).override_prefix_host();

// Stored as `__Host-session`, forced Secure + Path=/, no Domain.
jar.add(cookie!("session" = "abc123"));

// Looked up under the prefixed name; returns it with the prefix stripped.
let session = jar.get("session");
# }
```

As with attributes, each prefix has a `default_*` form that fills the required attributes only when unset, and an `override_*` form that forces them for guaranteed RFC compliance. Use `override_*` unless you have a reason to let a caller's value stand.

[RFC 6265bis]: https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis#name-cookie-name-prefixes

# Signed cookies

A **signed** cookie is tamper-proof but still readable by the client: useful when the value isn't secret but must not be forged (a user id, a feature flag). Signing wraps the jar with a [`Key`]; reads return `None` when a cookie is missing or its signature doesn't verify.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, Key, cookie, cookies};

let key = Key::generate();
let jar = cookies(cx).signed(&key);

jar.add(cookie!("user_id" = "42"));

// Returns the cookie only if the signature checks out.
let user_id = jar.get("user_id");
# }
```

# Private cookies

A **private** cookie is encrypted with AES-256-GCM, so its value is both tamper-proof *and* unreadable by the client. Use it for anything sensitive. The cookie's name is bound into the ciphertext, so the name must match on write and read, which it does automatically, however you compose this layer.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, Key, cookie, cookies};

let key = Key::generate();
let jar = cookies(cx).private(&key);

jar.add(cookie!("session" = "secret-token"));

let session = jar.get("session"); // None if missing or it fails to decrypt
# }
```

Signing and private encryption operate on the cookie value (and, for private, the name) only: they compose freely with prefixes and attribute defaults in any order.

# Keys from app context

In a real app you generate the [`Key`] once at startup and share it across requests. Register it as [app context](crate::context::app_context):

```rust
use topcoat::{
    cookie::{Key, RouterBuilderCookieExt},
    router::{Router, RouterBuilderDiscoverExt},
};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .cookies()
        .app_context(Key::generate())
        .build()
}
```

Then `signed_cookies(cx)` and `private_cookies(cx)` give you a wrapped jar using that registered key, with no plumbing:

```rust
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies, cookie, private_cookies},
    router::route,
};

#[route(POST "/api/login")]
async fn login(cx: &Cx) -> Result<&'static str> {
    private_cookies(cx).add(cookie!("session" = "secret-token"; Path = "/"));
    Ok("logged in")
}
```

Both functions panic if no [`Key`] was registered: a startup-time bug, not a runtime one. Generate the key once and persist it; regenerating it on every boot invalidates every signed and encrypted cookie already in the wild.

# Typed cookie stores

The jar API works in terms of individual [`Cookie`] values. When you want to keep a *structured* value in a cookie (a cart, a preferences object, a visit counter), a [`CookieStore<T>`](CookieStore) wraps the read/serialize/write cycle so you work with your own type instead of strings. The value is stored as JSON, and `T` only needs `Serialize` and `DeserializeOwned`.

A store is built on top of a jar with [`cookie_store`], so signing, encryption, prefixes, and default attributes all compose through the jar you hand it: a store over [`private_cookies`] is encrypted, a store over [`cookies(cx).signed(key)`](Cookies::signed) is signed, and so on.

```rust
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    cookie::{cookie_store, private_cookies},
    router::route,
};

#[derive(Default, Serialize, Deserialize)]
struct Cart {
    items: Vec<String>,
}

#[route(POST "/api/cart")]
async fn add_item(cx: &Cx) -> Result<String> {
    let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart")
        .parse_or_default()
        .update(|cart| cart.items.push("widget".to_owned()))
        .commit()?;

    Ok(format!("{} items in cart", cart.items.len()))
}
```

## Reading the incoming value

[`cookie_store`] returns an [`UnparsedCookieStore`]. Reading the incoming cookie is a separate, fallible step, because a cookie can be absent or present-but-malformed (for example after you change `T`'s shape). The `parse*` methods mirror [`Option`]/[`Result`]'s `unwrap*` family and let you choose how to handle those cases:

- [`parse`](UnparsedCookieStore::parse) returns `Ok(None)` when the cookie is absent and `Err` when it is present but won't deserialize, so you can distinguish the two.
- [`parse_or(value)`](UnparsedCookieStore::parse_or) falls back to `value` when the cookie is absent or malformed.
- [`parse_or_else(f)`](UnparsedCookieStore::parse_or_else) falls back to `f()`.
- [`parse_or_default()`](UnparsedCookieStore::parse_or_default) falls back to `T::default()`.

The `parse_or*` methods deliberately treat a malformed cookie the same as a missing one. Because you can't migrate a cookie that already lives on the client, this means a change to `T` resets stale cookies to the fallback instead of failing every returning visitor. Use [`parse`](UnparsedCookieStore::parse) when you need to surface corruption instead.

Once parsed, you hold a [`CookieStore<T>`](CookieStore) whose value is known, so reads and mutations no longer return [`Result`]:

- [`read`](CookieStore::read) borrows the value; [`get`](CookieStore::get) clones it (when `T: Clone`).
- [`set(value)`](CookieStore::set) replaces the value and [`update(f)`](CookieStore::update) mutates it in place. Both return the store so calls can be chained.

## Nothing is written until `commit`

Reads and mutations touch only the in-memory value. **No `Set-Cookie` is queued until you call [`commit`](CookieStore::commit)**, which serializes the value, writes it through the jar, and hands the value back:

```rust
# use topcoat::cookie::{cookie_store, private_cookies};
# #[derive(Default, serde::Serialize, serde::Deserialize)] struct Cart { items: Vec<String> }
# fn _example(cx: &topcoat::context::Cx) -> topcoat::Result<()> {
let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart")
    .parse_or_default()
    .update(|cart| cart.items.push("widget".to_owned()))
    .commit()?;
# let _ = cart;
# Ok(())
# }
```

Dropping the store without committing (or calling [`rollback`](CookieStore::rollback) to say so explicitly) discards the pending changes. This makes it easy to update a cookie only once some other work has succeeded: do the work first, and `commit` last.

To overwrite a cookie without reading its current contents, [`set`](UnparsedCookieStore::set) on the unparsed store skips the parse step entirely:

```rust
# use topcoat::cookie::{cookie_store, private_cookies};
# #[derive(Default, serde::Serialize, serde::Deserialize)] struct Cart { items: Vec<String> }
# fn _example(cx: &topcoat::context::Cx) -> topcoat::Result<()> {
cookie_store::<Cart, _>(private_cookies(cx), "cart")
    .set(Cart::default())
    .commit()?;
# Ok(())
# }
```

To delete a cookie, `remove` queues an expiring removal. It's available both on a parsed store ([`CookieStore::remove`]) and directly on the unparsed one ([`UnparsedCookieStore::remove`]) when you just want to clear the cookie without reading it, for example on logout:

```rust
# use topcoat::cookie::{cookie_store, private_cookies};
# #[derive(Default, serde::Serialize, serde::Deserialize)] struct Cart { items: Vec<String> }
# fn _example(cx: &topcoat::context::Cx) {
cookie_store::<Cart, _>(private_cookies(cx), "cart").remove();
# }
```

The removal goes through the jar, so the `Path`/`Domain` and prefix attributes the cookie was written with are reapplied: the browser matches the removal against the original and clears it.

## A helper per store

As with the jar combinators, the idiomatic pattern is to wrap each store in a small helper so its name and backing jar stay consistent everywhere it's used:

```rust
# #[derive(Default, serde::Serialize, serde::Deserialize)] struct Cart { items: Vec<String> }
use topcoat::{
    context::Cx,
    cookie::{CookieStore, Cookies, cookie_store, signed_cookies},
};

fn cart(cx: &Cx) -> CookieStore<Cart, impl Cookies> {
    cookie_store(signed_cookies(cx), "cart").parse_or_default()
}
```

Every handler then calls `cart(cx)`, mutates, and commits, without repeating the name or the jar configuration.
