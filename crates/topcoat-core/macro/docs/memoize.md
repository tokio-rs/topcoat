The `#[memoize]` attribute caches the result of a function for the duration of one request, keyed by its arguments and the request-context bindings it reads. A second call reuses a result when its arguments match and those observed bindings are unchanged.

This is the per-request equivalent of memoization in libraries like React's `cache`: it's not a global cache and it's not persisted across requests. Each new request starts with an empty cache.

# Setup

Annotate any function that takes a `cx: &Cx` parameter:

```rust
# fn main() {}
# struct User;
# mod db {
#     pub async fn load_user(_id: i64) -> super::User { super::User }
# }
use topcoat::context::{Cx, memoize};

#[memoize]
async fn get_user(cx: &Cx, id: i64) -> User {
    db::load_user(id).await
}
```

That's it. Calling `get_user(cx, 42).await` from anywhere in the request (a page, a layout, a component) runs the body the first time and returns the cached `User` for every subsequent call with `id == 42`. The function's return type `T` is rewritten to `&T` that has the same lifetime as `&cx`.

Top-level `Option<T>` and `Result<T, E>` return types are borrowed ergonomically: the macro calls `.as_ref()` on the cached value and returns `Option<&T>` or `Result<&T, &E>` instead of `&Option<T>` or `&Result<T, E>`.

# Sync and async

`#[memoize]` works on both synchronous and `async` functions. Pick whichever matches your work; the macro handles the rest.

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize};
# #[derive(serde::Deserialize)]
# struct Config;
# struct Post;
# mod db {
#     pub async fn load_post(_slug: &str) -> super::Post { super::Post }
# }
#[memoize]
fn parse_config(cx: &Cx, raw: &str) -> Config {
    serde_json::from_str(raw).unwrap()
}

#[memoize]
async fn fetch_post(cx: &Cx, slug: &str) -> Post {
    db::load_post(slug).await
}
```

For async functions, one body runs at a time for each argument key. Concurrent callers with the same context bindings share its result. A caller whose context resolves an observed type differently waits, checks the completed result, then computes and stores another variant when needed. Calls with different argument keys can run in parallel.

# What gets cached

Every argument except `cx` is part of the cache key. Two calls hit the same cache entry if and only if every non-`cx` argument is equal.

```rust
# use topcoat::context::{Cx, memoize};
#[memoize]
fn add(cx: &Cx, x: i32, y: i32) -> i32 {
    println!("computing");
    x + y
}

# fn example(cx: &Cx) {
add(cx, 1, 2); // prints "computing", returns 3
add(cx, 1, 2); // returns 3 from cache
add(cx, 1, 3); // prints "computing", returns 4 (different args)
# }
```

Each `#[memoize]` function has its own independent cache slot, so two functions with the same argument types don't collide.

# Request context dependencies

Calls to `request_context` and `try_request_context` become cache dependencies. An unrelated scoped value does not prevent reuse, while shadowing an observed type selects another result:

```rust
# use topcoat::context::{Cx, memoize, request_context};
#[derive(Clone, Copy)]
struct DocsVersion(&'static str);
struct HeadingLevel(u8);

#[memoize]
fn docs_path(cx: &Cx, page: &str) -> String {
    let version = request_context::<DocsVersion>(cx);
    format!("/{}/{page}", version.0)
}

# fn example(cx: &Cx) {
let stable = cx.with(DocsVersion("stable"));
let section = stable.with(HeadingLevel(2));
let next = stable.with(DocsVersion("next"));

docs_path(&stable, "router");  // computes stable
docs_path(&section, "router"); // reuses stable
docs_path(&next, "router");    // computes next
# }
```

Bindings are matched by identity, not `PartialEq`. Two sibling scopes that each contain `DocsVersion("stable")` hold distinct bindings and compute distinct variants. Create one shared scope above sibling calls when they should reuse a result.

An optional lookup that finds no value records that absence. Adding the type later prevents reuse of the result that observed it missing. App-context reads do not add dependencies because app context is immutable for the request.

Dependencies from nested memoized calls propagate to their caller, including when the nested call is a cache hit. Bindings created in a child scope inside the outer function are not outer inputs, so they are excluded from the outer result's dependencies.

Each outermost call captures a request-context revision before checking the cache or waiting. Its body and any waiter continue to resolve ancestor bindings at that revision if another task inserts a newer binding while work is in flight.

# Borrowed and owned arguments

Arguments can be passed by value or by reference. Borrowed arguments avoid cloning on cache hits; on a miss the value is cloned once into the cache.

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize};
# struct Record;
# struct Error;
# mod db {
#     pub async fn find(_name: &str) -> Result<super::Record, super::Error> { Ok(super::Record) }
# }
#[memoize]
async fn lookup(cx: &Cx, name: &str) -> Result<Record, Error> {
    db::find(name).await
}

# async fn example(cx: &Cx) -> Result<(), &Error> {
let record = lookup(cx, "alice").await?; // computes; stores "alice".to_owned() as the key
let record = lookup(cx, "alice").await?; // cache hit, no allocation
# let _ = record;
# Ok(())
# }
```

# Requirements

The macro enforces these at compile time:

- The function must take a parameter literally named `cx` of type `&Cx`.
- The function cannot take a `self` receiver.
- For an owned argument of type `P`: `P: Clone + Hash + Eq + Send + Sync + 'static`.
- For a borrowed argument of type `&P`: `P: ToOwned` with `P::Owned: Hash + Eq + Send + Sync + 'static`.
- The return type `T` must be `Send + Sync + 'static`.
- Calling `Cx::insert` from the memoized body panics. `Cx::with` and `Cx::with_values` remain available.

Most everyday types (`i32`, `String`, `&str`, `Uuid`, your own `#[derive(Hash, Eq, PartialEq, Clone)]` structs) satisfy these out of the box.

# When to reach for it

Use `#[memoize]` when the same data may be requested multiple times during a single request and recomputing it is wasteful. Common cases:

- **Database lookups** that several components need (current user, settings, feature flags).
- **Deduplication of fan-out fetches** when components render in parallel and would otherwise hit the same endpoint repeatedly.

It is *not* a substitute for a long-lived cache (Redis, an LRU, etc.). Cross-request caching is a separate concern and should be layered behind your data access functions.

# Example: shared user lookup

```rust
# fn main() {}
# struct User { name: String }
# mod auth {
#     pub async fn resolve(_cx: &topcoat::context::Cx) -> Option<super::User> { None }
# }
use topcoat::{
    context::{Cx, memoize},
    Result,
    router::{layout, page},
    view::view,
};

#[memoize]
async fn current_user(cx: &Cx) -> Option<User> {
    auth::resolve(cx).await
}

#[page]
async fn dashboard(cx: &Cx) -> Result {
    let user = current_user(cx).await; // computes once
    view! { <h1>"Welcome, " (user.unwrap().name.clone())</h1> }
}

#[layout]
async fn root(cx: &Cx, slot: Result) -> Result {
    let user = current_user(cx).await; // cache hit, no extra DB query
    view! {
        <header>
            match user {
                Some(u) => {
                    "Hello, " (u.name.clone())
                },
                None => <a href="/login">"Sign in"</a>,
            }
        </header>
        (slot?)
    }
}
```

The page renders before the layout it is wrapped in, so the page computes the user and the layout reads it from the cache. Either way the database is queried at most once per request.
