The `#[memoize]` attribute caches the result of a function for the duration of a single request, keyed by its arguments. If you call the same function twice with the same arguments during one request, the body runs only once and the second call returns the cached value.

This is similar to React's [`cache`](https://react.dev/reference/react/cache): the cache is not global and is not kept across requests. Each new request starts with an empty cache.

# Setup

Add the attribute to a function that takes a `cx: &Cx` parameter:

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

The first call to `get_user(cx, 42).await` during a request runs the body. Every later call with `id == 42` in the same request, whether from a page, a layout, or a component, returns the cached `User`.

The macro changes the return type from `T` to `&T`, borrowed for as long as `cx`. To borrow the contents of an `Option<T>` or `Result<T, E>` instead, see [`as_ref`](#borrowing-option-and-result-contents) below.

# Sync and async

`#[memoize]` works on both synchronous and `async` functions:

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

For an async function, concurrent calls with the same arguments run the body only once. If two parts of a page render at the same time and both call `fetch_post(cx, "hello")`, the database is queried once, and the second caller waits for the result of the first.

# Recursion

A memoized function can call itself, but only with different arguments. Each call with different arguments uses its own cache entry:

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

A nested call with the same arguments would wait for its own result forever, so it panics instead:

```rust,should_panic
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

Every argument except `cx` is part of the cache key. Two calls share a cache entry when all their other arguments are equal:

```rust
# use topcoat::context::{Cx, memoize};
#[memoize]
fn add(cx: &Cx, x: i32, y: i32) -> i32 {
    println!("computing");
    x + y
}

# fn example(cx: &Cx) {
add(cx, 1, 2); // prints "computing", returns 3
add(cx, 1, 2); // returns 3 from the cache
add(cx, 1, 3); // prints "computing", returns 4
# }
```

Each memoized function has its own cache entries, so two functions with the same argument types never share results.

The arguments are not the only thing that decides whether a call can reuse a cached result. The request context values the body reads matter too, as the next section explains.

# Request context dependencies

Request context values are not part of the cache key, but they still decide which callers can reuse a cached result. While the body runs, `#[memoize]` records every request context value it reads. A later call reuses the result only if its `cx` resolves those reads to the same values.

This matters because `Cx::with` creates a child scope with extra values, so the same function can see different values depending on the `cx` it is called with:

```rust
# fn main() {}
use topcoat::context::{Cx, memoize, request_context};

struct Locale(&'static str);

#[memoize]
fn greeting(cx: &Cx) -> String {
    format!("hello in {}", request_context::<Locale>(cx).0)
}

# fn example(cx: &Cx) {
greeting(cx); // runs the body, reading the request's Locale

let scoped = cx.with(Locale("de"));
greeting(&scoped); // sees a different Locale, so the body runs again
# }
```

The first call read the request's own `Locale`, so the call through `scoped` cannot reuse its result and runs the body again. Both results stay cached, and a later call through either scope reuses the matching one. Without this, a value set for one scope could leak out of it, and whichever call came first would decide the result for every later caller.

Only the values the body actually reads count. Registering a value the body never reads changes nothing, and a body that reads no request context can be reused from every scope. A lookup that finds nothing also counts: if the body looked for a type that was not registered, a caller whose scope registers that type runs the body again.

Values that the body registers itself are not dependencies:

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

That read goes through a scope the body created itself, which no caller can see, so it never keeps a caller from reusing the result.

Dependencies also carry over through nested memoized calls:

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

`greeting` never reads `Locale` itself, but it depends on it through `locale`. A caller that sets a different `Locale` runs both bodies again. This holds whether the nested call ran its body or returned a cached result.

App context values are not tracked. They are registered once for the whole application and cannot change per scope, so reading them never limits reuse.

# Borrowing Option and Result contents

By default the macro returns a reference to the cached value itself, so a function that returns `Option<User>` gives you `&Option<User>`. Pass `as_ref` to the attribute to borrow the contents of the cached value instead:

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

With `as_ref`, the return type goes through the `MemoizeAsRef` trait: `Option<T>` becomes `Option<&T>`, and `Result<T, E>` becomes `Result<&T, &E>`. Implement the trait for your own types to use them with `as_ref`.

# Borrowed and owned arguments

Arguments can be passed by value or by reference. The cache never stores the arguments. It identifies an entry by a hash of them, so borrowed arguments are hashed but never cloned.

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
let record = lookup(cx, "alice").await?; // runs the body and caches the result
let record = lookup(cx, "alice").await?; // reuses the result, without allocating
# let _ = record;
# Ok(())
# }
```

# Requirements

The macro checks the first two rules itself, and the compiler checks the others:

- The function must have a parameter named `cx` of type `&Cx`.
- The function cannot take a `self` receiver.
- Every argument except `cx` must implement `Hash`, whether it is passed by value or by reference.
- The return type must be `Send + Sync + 'static`.

Most common types, such as `i32`, `String`, `&str`, and your own `#[derive(Hash)]` structs, meet these rules.

The hash of the arguments is the whole cache key. The cache is only correct if every argument's `Hash` impl gives different results for values that are not equal. A hand-written impl that hashes only some fields of a type makes calls that differ in the other fields share an entry, and such a call silently returns the result of a different call. Derived impls and the impls in the standard library are always safe.

A memoized body cannot read the identity of its `cx` with `identity` or `try_identity`: doing so panics. If the result depends on the identity, read it before the call and pass it as an argument.

# When to use it

Use `#[memoize]` when several parts of a request need the same data and computing it again would be wasteful. Common cases are:

- Database lookups that several components need, such as the current user, settings, or feature flags.
- Fetches that components rendering at the same time would otherwise send to the same endpoint more than once.

It does not replace a long-lived cache like Redis or an LRU cache. Caching across requests is a separate concern that belongs behind your data access functions.

# Example: shared user lookup

```rust
# fn main() {}
# struct User { name: String }
# mod auth {
#     pub async fn resolve(_cx: &topcoat::context::Cx) -> Option<super::User> { None }
# }
use topcoat::{
    Result,
    context::{Cx, memoize},
    router::{Slot, layout, page},
    view::{View, view},
};

#[memoize(as_ref)]
async fn current_user(cx: &Cx) -> Option<User> {
    auth::resolve(cx).await
}

#[page]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let name = current_user(cx).await.map_or("guest", |user| user.name.as_str());
    Ok(view! { <h1>"Welcome, " (name)</h1> })
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let user = current_user(cx).await;
    Ok(view! {
        <header>
            match user {
                Some(user) => {
                    "Hello, " (user.name.as_str())
                },
                None => <a href="/login">"Sign in"</a>,
            }
        </header>
        (slot)
    })
}
```

The page and the layout both call `current_user`. Whichever runs first queries the database, and the other gets the cached result, so the database is queried at most once per request.
