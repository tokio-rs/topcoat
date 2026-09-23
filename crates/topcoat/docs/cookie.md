Topcoat reads and writes [HTTP cookies](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Cookies) through a cookie jar that belongs to the current request. Add cookie support to your router with `.cookies()`, then call [`cookies(cx)`](cookies) in any handler to get the jar. The jar holds the cookies the request carried. Any cookie you add or remove is sent as a `Set-Cookie` response header when the handler returns, whether it returns a response or an error such as a redirect.

Cookies are part of the default feature set, and everything below lives in `topcoat::cookie`. Cookies themselves come from the [`cookie`](https://docs.rs/cookie) crate: a cookie is a [`Cookie`], and signing and encryption use its [`Key`].

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

[`cookies(cx)`](cookies) returns the request's root jar. The first call parses the request's `Cookie` headers, and later calls in the same request return the same jar. Bring the [`Cookies`] trait into scope to use its `get`, `add`, and `remove` methods.

Build a [`Cookie`] with `Cookie::new`, or with `Cookie::build` when it needs attributes:

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

- [`get(name)`](Cookies::get) returns the cookie with that name, or `None` if the request did not carry it.
- [`add(cookie)`](Cookies::add) sets a cookie.
- [`remove(cookie)`](Cookies::remove) deletes a cookie by sending an expired cookie with the same name. The browser only deletes the cookie if the `Path` and `Domain` match the ones it was set with, so pass the same ones:

```rust
# use topcoat::cookie::{Cookie, Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
# let jar = cookies(cx);
jar.remove(Cookie::build(("session", "")).path("/").build());
# }
```

`add` and `remove` accept anything that converts into a [`Cookie`], so a `(name, value)` tuple works when you do not need attributes:

```rust
# use topcoat::cookie::{Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
# let jar = cookies(cx);
jar.add(("theme", "dark"));
# }
```

# Writes must happen before the response

Cookies are sent in the response headers, so they cannot change once the headers are sent. Some content keeps running after the handler returns: a WebSocket task, a [`live!`](crate::view::live) region, or the content of a [`suspense`](crate::view::suspense) boundary. Such content can read cookies, but adding or removing a cookie there panics.

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
    // The handler has not returned yet, so this write is sent.
    cookies(cx).add(("last_report", "sales"));

    Ok(view! {
        suspense(
            fallback: view! { <p>"Loading..."</p> },
            // Streams in after the handler returns, so it must not write cookies.
            figures()
        )
    })
}
```

# Building cookies with `cookie!`

The [`cookie!`] macro is a shorter way to build a cookie with several attributes. Its syntax mirrors the [`Set-Cookie`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie) header: the `name = value` pair comes first, followed by attributes separated by `;`.

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

To avoid repeating the same attributes on every cookie, set them on the jar with the [`Cookies`] combinators. Like [`Iterator`] adapters, each combinator wraps the jar in a new jar, which applies an attribute to every cookie that passes through it. Each attribute has two combinators:

- `default_*` sets the attribute only when the cookie does not set it.
- `override_*` always sets the attribute, replacing the cookie's own value.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, SameSite, cookie, cookies};

let jar = cookies(cx)
    .default_secure(true)
    .default_http_only(true)
    .default_same_site(SameSite::Lax)
    .default_path("/");

// Gets Secure, HttpOnly, SameSite=Lax, and Path=/ from the defaults above.
jar.add(cookie!("session" = "abc123"));
# }
```

There are combinators for `Secure`, `HttpOnly`, `SameSite`, `Path`, `Domain`, and `Max-Age`. They also apply to removals, so a removal gets the same `Path` and `Domain` as the cookie it deletes. You can build a configured jar once and use it for several writes.

A good pattern is to write your own `cookies` function with your app's defaults, and use it everywhere instead of the one from `topcoat::cookie`:

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

Every handler that calls this function gets the defaults, and you can change them in one place. The same works for signed and private jars: return `impl Cookies` and add the combinators your app needs.

For changes that the attribute combinators do not cover, [`map`](Cookies::map) runs a closure on every cookie:

```rust
# use topcoat::cookie::{Cookies, cookies};
# fn _example(cx: &topcoat::context::Cx) {
let jar = cookies(cx).map(|cookie| cookie.set_partitioned(true));
# }
```

# Name prefixes

A browser only accepts a cookie whose name starts with one of the [RFC 6265bis] name prefixes if the cookie has the attributes the prefix requires. Topcoat adds the prefix and the required attributes for you. When reading, it looks up the prefixed name and returns the cookie under its bare name, so your code only uses the bare name.

- `__Host-`: the cookie must be `Secure`, have `Path=/`, and have no `Domain`. The cookie is bound to the exact host and is not sent to subdomains.
- `__Secure-`: the cookie must be `Secure`.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, cookie, cookies};

let jar = cookies(cx).override_prefix_host();

// Sent as `__Host-session` with Secure and Path=/, and without Domain.
jar.add(cookie!("session" = "abc123"));

// Looks up `__Host-session` and returns it named `session`.
let session = jar.get("session");
# }
```

Like the attribute combinators, each prefix has two forms. [`override_prefix_host`](Cookies::override_prefix_host) and [`override_prefix_secure`](Cookies::override_prefix_secure) always set the required attributes, so the browser always accepts the cookie. [`default_prefix_host`](Cookies::default_prefix_host) and [`default_prefix_secure`](Cookies::default_prefix_secure) only set the attributes the cookie does not set itself. Prefer the `override_*` forms unless you need to keep a value set by the caller.

[RFC 6265bis]: https://datatracker.ietf.org/doc/html/draft-ietf-httpbis-rfc6265bis#name-cookie-name-prefixes

# Signed cookies

A signed cookie cannot be changed by the client, but the client can still read its value. Use it for values that are not secret but must not be forged, such as a user id. [`signed`](Cookies::signed) wraps the jar with a [`Key`]. Reads return `None` when the cookie is missing or its signature does not verify.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, Key, cookie, cookies};

let key = Key::generate();
let jar = cookies(cx).signed(&key);

jar.add(cookie!("user_id" = "42"));

// Returns the cookie only if the signature verifies.
let user_id = jar.get("user_id");
# }
```

# Private cookies

A private cookie is encrypted with AES-256-GCM, so the client can neither change nor read its value. Use it for anything sensitive. [`private`](Cookies::private) wraps the jar with a [`Key`]. Reads return `None` when the cookie is missing or fails to decrypt.

```rust
# fn _example(cx: &topcoat::context::Cx) {
use topcoat::cookie::{Cookies, Key, cookie, cookies};

let key = Key::generate();
let jar = cookies(cx).private(&key);

jar.add(cookie!("session" = "secret-token"));

// `None` if the cookie is missing or fails to decrypt.
let session = jar.get("session");
# }
```

Signed and private jars combine with name prefixes and attribute combinators in any order. The encryption of a private cookie also covers its name, but this needs no care from you as long as you read the cookie through the same combinators you wrote it with.

# Keys from app context

In an application, create the [`Key`] once at startup and share it across requests by registering it as [app context](crate::context::app_context):

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

[`signed_cookies(cx)`](signed_cookies) and [`private_cookies(cx)`](private_cookies) then return the root jar wrapped with that key:

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

Both functions panic if no [`Key`] is registered. A key from `Key::generate()` changes on every restart, which makes every signed and private cookie issued before the restart invalid. In production, load the key from a secret that stays the same across restarts instead:

```rust
use topcoat::cookie::Key;

/// Builds the cookie key from a secret of at least 64 bytes.
fn cookie_key(secret: &[u8]) -> Key {
    Key::try_from(secret).expect("the cookie secret should be at least 64 bytes")
}
```

# Typed cookie stores

The jar works with individual [`Cookie`] values, whose values are strings. To keep a structured value in a cookie, such as a cart or user preferences, use a [`CookieStore<T>`](CookieStore). It reads, deserializes, serializes, and writes the cookie for you, so you work with your own type. The value is stored as JSON, and `T` must implement `Serialize` and `DeserializeOwned`.

Create a store for a jar with [`cookie_store`]. The store reads and writes its cookie through that jar, so a store over [`private_cookies`] is encrypted, a store over a [`signed`](Cookies::signed) jar is signed, and so on.

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

[`cookie_store`] returns an [`UnparsedCookieStore`]. Reading the cookie is a separate step, because the cookie can be missing, or present but impossible to deserialize, for example after you change the shape of `T`. The `parse*` methods work like the `unwrap*` methods of [`Option`] and [`Result`], and let you choose how to handle these cases:

- [`parse`](UnparsedCookieStore::parse) returns `Ok(None)` when the cookie is missing and `Err` when it cannot be deserialized, so you can tell the two apart.
- [`parse_or(value)`](UnparsedCookieStore::parse_or) uses `value` when the cookie is missing or cannot be deserialized.
- [`parse_or_else(f)`](UnparsedCookieStore::parse_or_else) calls `f` for the value instead.
- [`parse_or_default()`](UnparsedCookieStore::parse_or_default) uses `T::default()`.

The `parse_or*` methods treat a cookie that cannot be deserialized the same as a missing one. You cannot migrate cookies that are already stored in browsers, so after a change to `T`, old cookies fall back to the default instead of causing an error for every returning visitor. Use [`parse`](UnparsedCookieStore::parse) when you need to detect such cookies.

After parsing, you have a [`CookieStore<T>`](CookieStore) that holds a value, so reading and changing it cannot fail:

- [`read`](CookieStore::read) borrows the value, and [`get`](CookieStore::get) clones it when `T: Clone`.
- [`set(value)`](CookieStore::set) replaces the value, and [`update(f)`](CookieStore::update) changes it in place. Both return the store, so calls can be chained.

## Nothing is written until `commit`

Reading and changing the store only affects the value in memory. The cookie is only written when you call [`commit`](CookieStore::commit), which serializes the value, adds it to the jar, and returns the value:

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

Dropping the store without calling `commit` discards the changes. [`rollback`](CookieStore::rollback) does the same, but makes the intent clear. To update a cookie only after some other work succeeds, do that work first and call `commit` last.

To replace a cookie without reading it, call [`set`](UnparsedCookieStore::set) on the unparsed store:

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

To delete the cookie, call `remove`. It exists on a parsed store ([`CookieStore::remove`]) and on an unparsed one ([`UnparsedCookieStore::remove`]), which is useful when you do not need the current value, for example on logout:

```rust
# use topcoat::cookie::{cookie_store, private_cookies};
# #[derive(Default, serde::Serialize, serde::Deserialize)] struct Cart { items: Vec<String> }
# fn _example(cx: &topcoat::context::Cx) {
cookie_store::<Cart, _>(private_cookies(cx), "cart").remove();
# }
```

The removal goes through the jar, which applies the same `Path`, `Domain`, and name prefix as when the cookie was written, so the browser matches the removal and deletes the cookie.

## A helper per store

As with the jar, a good pattern is to wrap each store in a small function, so its cookie name and jar are the same everywhere:

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

Handlers then call `cart(cx)`, change the value, and commit it, without repeating the cookie name or the jar setup.
