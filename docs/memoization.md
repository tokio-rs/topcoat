# Memoization

The `#[memoize]` attribute caches the result of a function for the duration of a single request, keyed by its arguments. Call the same function twice with the same arguments inside one request and the body runs only once — the second call returns the cached value.

This is the per-request equivalent of memoization in libraries like React's `cache`: it's not a global cache and it's not persisted across requests. Each new request starts with an empty cache.

## Setup

Annotate any function that takes a `cx: &Cx` parameter:

```rust
use topcoat::context::{Cx, memoize};

#[memoize]
async fn get_user(cx: &Cx, id: i64) -> User {
    db::load_user(id).await
}
```

That's it. Calling `get_user(cx, 42).await` from anywhere in the request — a page, a layout, a component — runs the body the first time and returns the cached `User` for every subsequent call with `id == 42`.

## Sync and async

`#[memoize]` works on both synchronous and `async` functions. Pick whichever matches your work; the macro handles the rest.

```rust
#[memoize]
fn parse_config(cx: &Cx, raw: &str) -> Config {
    serde_json::from_str(raw).unwrap()
}

#[memoize]
async fn fetch_post(cx: &Cx, slug: &str) -> Post {
    db::load_post(slug).await
}
```

For async functions, concurrent callers with the same arguments share a single in-flight future. If two parts of your page render in parallel and both call `fetch_post(cx, "hello")`, the database is queried once and both callers await the same result.

## What gets cached

Every argument except `cx` is part of the cache key. Two calls hit the same cache entry if and only if every non-`cx` argument is equal.

```rust
#[memoize]
fn add(cx: &Cx, x: i32, y: i32) -> i32 {
    println!("computing");
    x + y
}

add(cx, 1, 2); // prints "computing", returns 3
add(cx, 1, 2); // returns 3 from cache
add(cx, 1, 3); // prints "computing", returns 4 (different args)
```

Each `#[memoize]` function has its own independent cache slot, so two functions with the same argument types don't collide.

## Borrowed and owned arguments

Arguments can be passed by value or by reference. Borrowed arguments avoid cloning on cache hits; on a miss the value is cloned once into the cache.

```rust
#[memoize]
async fn lookup(cx: &Cx, name: &str) -> Result<Record, Error> {
    db::find(name).await
}

lookup(cx, "alice").await; // computes; stores "alice".to_owned() as the key
lookup(cx, "alice").await; // cache hit, no allocation
```

## The `Memoized<T>` return type

`#[memoize]` rewrites the function to return [`Memoized<'_, T>`] instead of `T`. This is a smart-pointer handle that:

- dereferences to `&T`, so you can use it anywhere a reference works,
- is tied to the lifetime of the request context, so it can't outlive the cache it came from.

```rust
#[memoize]
async fn site_title(cx: &Cx) -> String {
    settings::load_title(cx).await
}

let title = site_title(cx).await;     // Memoized<'_, String>
println!("{}", *title);               // deref to &String
let owned: String = (*title).clone(); // clone if you need ownership
```

If you need to send the value across a `'static` boundary (e.g. into a spawned task), `clone()` the inner value out — don't try to move the `Memoized` itself.

## Requirements

The macro enforces these at compile time:

- The function must take a parameter literally named `cx` of type `&Cx`.
- The function cannot take a `self` receiver.
- For an owned argument of type `P`: `P: Clone + Hash + Eq + Send + Sync + 'static`.
- For a borrowed argument of type `&P`: `P: ToOwned` with `P::Owned: Hash + Eq + Send + Sync + 'static`.
- The return type `T` must be `Send + Sync + 'static`.

Most everyday types — `i32`, `String`, `&str`, `Uuid`, your own `#[derive(Hash, Eq, PartialEq, Clone)]` structs — satisfy these out of the box.

## When to reach for it

Use `#[memoize]` when the same data may be requested multiple times during a single request and recomputing it is wasteful. Common cases:

- **Database lookups** that several components need (current user, settings, feature flags).
- **Deduplication of fan-out fetches** when components render in parallel and would otherwise hit the same endpoint repeatedly.

It is *not* a substitute for a long-lived cache (Redis, an LRU, etc.). Cross-request caching is a separate concern and should be layered behind your data access functions.

## Example: shared user lookup

```rust
use topcoat::{
    context::{Cx, memoize},
    router::{Slot, layout, page},
    view::{View, view},
};

#[memoize]
async fn current_user(cx: &Cx) -> Option<User> {
    auth::resolve(cx).await
}

#[layout]
async fn root(cx: &Cx, slot: Slot) -> View {
    let user = current_user(cx).await; // computes once
    view! {
        <header>
            (match &*user {
                Some(u) => view! { "Hello, " (u.name.clone()) },
                None => view! { <a href="/login">"Sign in"</a> },
            })
        </header>
        (slot.await)
    }
}

#[page]
async fn dashboard(cx: &Cx) -> View {
    let user = current_user(cx).await; // cache hit, no extra DB query
    view! { <h1>"Welcome, " (user.as_ref().unwrap().name.clone())</h1> }
}
```

The layout and the page each call `current_user(cx)`, but the database is queried at most once per request.

