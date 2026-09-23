`#[memoize]` shares a function's result across calls within a request. Calls with the same arguments reuse the result when the request context values they depend on also match.

Each request has its own cache.

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

Calling `get_user(cx, 42).await` returns a reference to the cached `User`. The macro changes the return type from `T` to `&T`, with the same lifetime as `&cx`. Use [`as_ref`](#borrowing-option-and-result-contents) to borrow the contents of an `Option` or `Result`.

# Sync and async

`#[memoize]` supports synchronous and `async` functions:

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

Concurrent async calls that use the same cache entry wait for the same computation.

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

Every argument except `cx` contributes to the cache key through its `Hash` implementation. Calls also depend on the request context values they read, as described below.

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

Each function has its own cache entries.

# Request context dependencies

`#[memoize]` tracks the request context values read by the function. A caller can reuse the result only when those lookups resolve to the same registered values.

`Cx::with` can give the function a different value:

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

Each scope keeps a result for its own `Locale`. Repeated calls through either scope reuse the matching result.

Only the values the body actually reads count. Registering a value the body never looks up changes nothing, and a body that reads no request context at all is reusable from every scope. A lookup that found nothing is a dependency too: if the body read a type that was absent, a caller whose scope registers that type recomputes.

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

The function chooses its own `Locale`, so callers can share this result regardless of their scope's `Locale`.

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

With `as_ref`, `Option<T>` becomes `Option<&T>` and `Result<T, E>` becomes `Result<&T, &E>`. Implement `MemoizeAsRef` to support another return type.

# Borrowed and owned arguments

Arguments can be owned or borrowed. They must implement `Hash`, but do not need to implement `Clone`.

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
let record = lookup(cx, "alice").await?; // computes
let record = lookup(cx, "alice").await?; // reuses the result
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

Cache keys use argument hashes without comparing the original values. A collision can return another call's result. Include every field that affects the result in a custom `Hash` implementation. Deriving `Hash` includes all fields.

# When to reach for it

Use `#[memoize]` when repeated calls within a request should share work, such as a database lookup. Put any cache that needs to survive across requests in your data access code.

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

Both callers share the cached user lookup within the request.
