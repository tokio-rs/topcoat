Use a cookie jar to read cookies and queue changes for the response. Register cookie support with `.cookies()`, then call [`cookies`] from a handler. Changes become `Set-Cookie` headers on successful responses and error responses.

Cookie support is enabled by default. Its API is available in `topcoat::cookie`.

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

`cookies(cx)` returns the same jar throughout the request. Import the [`Cookies`] trait to read, add, or remove cookies.

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

# Writes must happen before the response

Cookie changes must happen before the router finishes the response headers. Later writes, including writes from content that streams after the initial response, panic. Reading cookies remains allowed.

```rust
use topcoat::{
    Result,
    context::Cx,
    cookie::{Cookies, cookies},
    router::page,
    view::{View, suspense, view},
};
# use topcoat::view::component;
# #[component]
# async fn figures() -> Result<impl View> { Ok(view! { <p>"42"</p> }) }

#[page("/report")]
async fn report(cx: &Cx) -> Result<impl View> {
    // Runs while the handler is still in charge of the response.
    cookies(cx).add(("last_report", "sales"));

    Ok(view! {
        suspense(
            fallback: view! { <p>"Loading..."</p> },
            // Streams in later, so it must not touch the jar.
            figures()
        )
    })
}
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

Configure attributes on a jar to apply them to each cookie written through it:

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

Reuse the configured jar for cookies that share the same settings.

Put shared settings in a helper:

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

Handlers can call this helper to use the same settings.

Use [`map`](Cookies::map) to apply a custom change to each added cookie:

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

A signed jar checks that a cookie was written with the same [`Key`]. The client can read the value, but a changed value fails verification. Reads return `None` for a missing cookie or an invalid signature.

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

A private jar encrypts and authenticates cookie values with a [`Key`]. Reads return `None` when the cookie is missing or cannot be decrypted and verified. The cookie name is also authenticated, so copying a value under a different name does not make it valid.

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

Register a shared [`Key`] as [app context](crate::context::app_context). This example generates a key for one process:

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

Then [`signed_cookies`] and [`private_cookies`] use the registered key:

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

Both functions panic if no [`Key`] was registered. To keep cookies valid across restarts and application instances, load the same persisted secret key instead of generating a new one at startup.

# Typed cookie stores

Use [`CookieStore<T>`](CookieStore) to read and write a structured value as JSON. The type must implement `Serialize` and `DeserializeOwned`.

Build the store with [`cookie_store`]. It uses the supplied jar's protection and attributes.

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

The `parse_or*` methods treat malformed and missing cookies alike. Use [`parse`](UnparsedCookieStore::parse) when you need to distinguish them.

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
