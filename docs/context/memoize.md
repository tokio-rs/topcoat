`#[memoize]` shares a function's result across calls in one request. A later call reuses the result when its arguments and observed request context bindings match.

Each request starts with an empty cache. Results are not shared across requests.

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

The first call to `get_user(cx, 42).await` computes the user. Later calls with the same ID reuse it if their request context dependencies match. The macro changes the return type from `T` to `&T`, borrowed for the lifetime of `cx`. Use [`as_ref`](#borrowing-option-and-result-contents) to borrow the contents of an `Option` or `Result` instead.

# Sync and async

The attribute works on synchronous and `async` functions.

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

For async functions, concurrent callers that resolve to the same cache entry share a single in-flight future. If two parts of your page render in parallel and both call `fetch_post(cx, "hello")`, the database is queried once and both callers await the same result.

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

Every argument except `cx` is hashed into the cache key. The function's request context dependencies also determine whether a cached result can be reused.

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

Each memoized function has a separate cache.

See [request context dependencies](#request-context-dependencies) for how scoped values affect reuse.

# Request context dependencies

The macro tracks request context lookups made while the function runs. A cached result can be reused only when those lookups resolve to the same registered bindings in the caller's scope.

This matters because `Cx::with` derives a child scope holding additional values, so the same function can see a different value depending on the `cx` it is called with:

```rust
# fn main() {}
use topcoat::context::{Cx, memoize, request_context};

struct Locale(&'static str);

#[memoize]
fn greeting(cx: &Cx) -> String {
    format!("hello in {}", request_context::<Locale>(cx).0)
}

# fn example(cx: &Cx) {
greeting(cx); // computes, reading the Locale registered for the request

let scoped = cx.with(Locale("de"));
greeting(&scoped); // a different Locale, so the body runs again
# }
```

The two calls read different `Locale` bindings, so each computes a result. Both results stay cached for later calls through their respective scopes.

Only observed lookups affect reuse. Adding an unrelated value does not invalidate the result. A lookup that returned `None` is also tracked. If a later caller has a value for that type, the function runs again.

Values the body registers itself are not dependencies:

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize, request_context};
# struct Locale(&'static str);
#[memoize]
fn banner(cx: &Cx) -> String {
    let scoped = cx.with(Locale("en"));
    format!("banner in {}", request_context::<Locale>(&scoped).0)
}
```

The function creates this binding itself, so it does not depend on the caller's value of that type.

Dependencies also propagate through nested memoized calls:

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize, request_context};
# struct Locale(&'static str);
#[memoize]
fn locale(cx: &Cx) -> String {
    request_context::<Locale>(cx).0.to_owned()
}

#[memoize]
fn greeting(cx: &Cx) -> String {
    format!("hello in {}", locale(cx))
}
```

`greeting` never reads `Locale` itself, but it depends on it through `locale`, so a caller that scopes a different `Locale` recomputes both. This holds whether the nested call computed or hit the cache.

App context is not tracked. It is registered once for the application and cannot be scoped per request, so reading it never constrains reuse.

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

Arguments can be passed by value or by reference. The cache never stores a copy of the arguments: an entry is identified by a hash of them, so borrowed arguments are only hashed, never cloned.

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
let record = lookup(cx, "alice").await?; // computes; caches under the hash of "alice"
let record = lookup(cx, "alice").await?; // cache hit, no allocation
# let _ = record;
# Ok(())
# }
```

# Requirements

The macro enforces these at compile time:

- The function must take a parameter literally named `cx` of type `&Cx`.
- The function cannot take a `self` receiver.
- Every argument except `cx` must implement `Hash`, whether passed by value or by reference.
- The return type `T` must be `Send + Sync + 'static`.

The cache identifies arguments by their hash without an equality check. A custom `Hash` implementation must include every part of the value that affects the result. Omitting such a field can make distinct calls reuse the wrong value. Prefer deriving `Hash` when possible.

# When to reach for it

Use `#[memoize]` when several callers need the same expensive result during one request. A shared user lookup is a typical example. Add a separate cache in your data access code if results should persist across requests.

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
    router::{Slot, layout, page},
    view::{View, view},
};

#[memoize(as_ref)]
async fn current_user(cx: &Cx) -> Option<User> {
    auth::resolve(cx).await
}

#[page]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let user = current_user(cx).await; // computes once
    Ok(view! { <h1>"Welcome, " (user.unwrap().name.clone())</h1> })
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = current_user(cx).await; // cache hit, no extra DB query
    Ok(view! {
        <header>
            match user {
                Some(u) => {
                    "Hello, " (u.name.clone())
                },
                None => <a href="/login">"Sign in"</a>,
            }
        </header>
        (slot)
    })
}
```

Whichever caller runs first loads the user. Later callers reuse the cached result, so the database is queried at most once per request.
