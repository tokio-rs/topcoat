`#[memoize]` saves a function's result so other calls in the same request can reuse it. Calls share a result when their arguments match and the context values read by the function have not been replaced.

The cache belongs to one request. Every new request starts with an empty cache.

# Setup

Add the attribute to a function with a `cx: &Cx` parameter:

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

The first call to `get_user(cx, 42).await` loads the user. Later calls with the same ID can borrow that result if the context values it depends on still match. The macro turns the return type `T` into `&T`. The reference lives as long as the borrowed `cx`. For an `Option` or `Result`, [`as_ref`](#borrowing-option-and-result-contents) lets callers borrow the value inside it instead.

# Sync and async

You can memoize both synchronous and `async` functions:

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

Concurrent async calls share the work when they can use the same cached result. If two parts of a page call `fetch_post(cx, "hello")` with matching context values, both wait for one database query.

# Recursion

Memoized functions can call themselves with different arguments. Calling back into the same unfinished computation with the same arguments panics.

Each set of arguments gets its own cache entry, so this recursion works:

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

A recursive call with the same arguments would wait for itself to finish. Topcoat panics instead:

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

The cache key includes a hash of every argument except `cx`. Before reusing a result, the cache also checks the context values that the function read.

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

Results from one memoized function are kept separate from those of another.

The [context rules below](#context-dependencies) explain when adding a value makes a call run again.

# Context dependencies

While the function runs, Topcoat records which context values it reads. A later call can reuse the result only if those reads would find the same registered values in its context. Registering a value again counts as a replacement, even if the new value is equal to the old one.

`Cx::with` can override a value in a child context. A memoized function called with that child may therefore need to run again:

```rust
# fn main() {}
use topcoat::context::{Cx, memoize, context};

struct Locale(&'static str);

#[memoize]
fn greeting(cx: &Cx) -> String {
    format!("hello in {}", context::<Locale>(cx).0)
}

# fn example(cx: &Cx) {
greeting(cx); // computes, reading the Locale visible to `cx`

let scoped = cx.with(Locale("de"));
greeting(&scoped); // a different Locale, so the body runs again
# }
```

These calls read different registered `Locale` values, so the function runs for each one. The cache keeps both results. Later calls use the one that matches their context.

Only types the function looked up matter. Adding a value of another type does not change which result is reused. A lookup that found nothing also matters. If a later call has a value of that type, the function runs again.

Values added by the function itself do not make the result depend on the caller's context:

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize, context};
# struct Locale(&'static str);
#[memoize]
fn banner(cx: &Cx) -> String {
    let scoped = cx.with(Locale("en"));
    format!("banner in {}", context::<Locale>(&scoped).0)
}
```

Here, `banner` always supplies its own `Locale`. Changing the caller's `Locale` does not change the result.

A function also depends on the context values read by memoized functions it calls:

```rust
# fn main() {}
# use topcoat::context::{Cx, memoize, context};
# struct Locale(&'static str);
#[memoize]
fn locale(cx: &Cx) -> String {
    context::<Locale>(cx).0.to_owned()
}

#[memoize]
fn greeting(cx: &Cx) -> String {
    format!("hello in {}", locale(cx))
}
```

`greeting` reads `Locale` indirectly through `locale`. Overriding `Locale` makes both functions run again for that context. Topcoat records this dependency even when `locale` returns a cached result.

Router values follow the same rules. Calls in a request can share a result that reads a router value. If a child context overrides that type with `Cx::with`, calls using the child need a result for the replacement value.

# Borrowing Option and Result contents

By default, callers borrow the entire cached result. For example, `Option<User>` becomes `&Option<User>`. Add `as_ref` to return `Option<&User>` instead:

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

`as_ref` uses the `MemoizeAsRef` trait to choose the borrowed return type. It changes `Option<T>` to `Option<&T>` and `Result<T, E>` to `Result<&T, &E>`. You can implement the trait for your own types too.

You can use `?` on a borrowed `&topcoat::Error` because it converts to an owned `topcoat::Error` by cloning. For other error types, you may need to convert the reference yourself, for example with `.map_err(Clone::clone)`.

# Borrowed and owned arguments

Arguments may be owned or borrowed. The cache stores their hash rather than the arguments themselves. Borrowed arguments do not need to be cloned:

```rust
# fn main() {}
# use topcoat::{context::{Cx, memoize}, Result};
# struct Record;
# mod db {
#     pub async fn find(_name: &str) -> topcoat::Result<super::Record> { Ok(super::Record) }
# }
#[memoize(as_ref)]
async fn lookup(cx: &Cx, name: &str) -> Result<Record> {
    db::find(name).await
}

# async fn example(cx: &Cx) -> Result<()> {
let record = lookup(cx, "alice").await?; // computes; caches under the hash of "alice"
let record = lookup(cx, "alice").await?; // cache hit, no allocation
# let _ = record;
# Ok(())
# }
```

# Requirements

These requirements are checked at compile time:

- Include a parameter named `cx` with type `&Cx`.
- Use a function without a `self` parameter.
- Make every argument other than `cx` hashable with `Hash`.
- Return a value that is `Send + Sync + 'static`.

The cache compares argument hashes, not the arguments themselves. If you write a `Hash` implementation, include every field that can affect the result. Otherwise, different calls may receive the same cached result. Prefer deriving `Hash` when you can.

# When to reach for it

Use `#[memoize]` for expensive work needed in several places during a request, such as looking up the current user. To keep results between requests, use a separate cache in the code that loads the data.

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

The first caller loads the user. Other callers with the same context dependencies borrow the saved result, so they do not repeat the database query.
