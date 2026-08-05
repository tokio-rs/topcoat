The `#[memoize]` attribute caches the result of a function for the duration of a single request, keyed by its arguments. Call the same function twice with the same arguments inside one request and the body runs only once: the second call returns the cached value.

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

That's it. Calling `get_user(cx, 42).await` from anywhere in the request (a page, a layout, a component) runs the body the first time and returns the cached `User` for every subsequent call with `id == 42`. The function's return type `T` is rewritten to `&T` that has the same lifetime as `&cx`. To borrow the contents of an `Option<T>` or `Result<T, E>` return value instead, see [`as_ref`](#borrowing-option-and-result-contents) below.

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

For async functions, concurrent callers with the same arguments share a single in-flight future. If two parts of your page render in parallel and both call `fetch_post(cx, "hello")`, the database is queried once and both callers await the same result.

# Recursion

Memoized functions can recurse if and only if recursive calls use different arguments. Recursion with identical arguments panics.

Recursion with different arguments uses a different cache entry for each call:

```rust
# use topcoat::context::{Cx, memoize};
#[memoize]
fn factorial(cx: &Cx, n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        _ => n * *factorial(cx, n - 1),
    }
}

# fn example(cx: &Cx) {
assert_eq!(*factorial(cx, 5), 120);
# }
```

A nested call with the same arguments panics because it would otherwise deadlock:

```should_panic
use topcoat::context::{Cx, memoize};

#[memoize]
fn recurse(cx: &Cx, n: u64) -> u64 {
    *recurse(cx, n)
}

fn main() {
    recurse(&Cx::default(), 1);
}
```

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

# Borrowing Option and Result contents

By default the macro returns a reference to the cached value itself: a function returning `Option<User>` hands out `&Option<User>`. Pass `as_ref` to the attribute to borrow the cached value's contents instead:

```rust
# fn main() {}
# struct User;
# mod db {
#     pub async fn load_user(_id: i64) -> Option<super::User> { None }
# }
use topcoat::context::{Cx, memoize};

#[memoize(as_ref)]
async fn find_user(cx: &Cx, id: i64) -> Option<User> {
    db::load_user(id).await
}

# async fn example(cx: &Cx) {
let user: Option<&User> = find_user(cx, 42).await;
# let _ = user;
# }
```

With `as_ref`, the macro rewrites the return type through the `MemoizeAsRef` trait: `Option<T>` comes back as `Option<&T>` and `Result<T, E>` as `Result<&T, &E>`. Implement the trait for your own return types to use them with `as_ref`.

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
#[memoize(as_ref)]
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

#[memoize(as_ref)]
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
