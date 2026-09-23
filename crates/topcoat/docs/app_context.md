# App context

Most apps need values that live longer than a single request, like a database pool, an HTTP client, or configuration loaded at startup. Topcoat stores these values in the **app context**. You register each value once on the router, and any code with access to the request context `Cx` can read it with `app_context(cx)`.

The app context is keyed by Rust type. Each type can be registered at most once, and lookups are typed: ask for a `&Database` and you get a `&Database`.

## Registering values

Call `.app_context(value)` on the router builder for every value you want to share:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .app_context(Database::connect())
        .app_context(HttpClient::new())
        .build()
}
```

Each value is stored under its concrete type. Registering a second value of the same type panics. If you need more than one value of the same type, wrap each one in its own newtype:

```rust
struct PrimaryDb(Database);
struct ReplicaDb(Database);

Router::builder()
    .app_context(PrimaryDb(Database::connect_primary()))
    .app_context(ReplicaDb(Database::connect_replica()))
    .build();
```

To check whether a value is already registered before adding one, call `get_app_context::<T>()` on the builder. It returns `None` when there is no value of that type yet.

## Reading values

In any page, layout, component, route, or helper function that has a `cx: &Cx`, call `app_context::<T>(cx)` to borrow the registered value:

```rust
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::page,
    view::{View, view},
};

#[page("/profile")]
async fn user_profile(cx: &Cx) -> Result<impl View> {
    let db: &Database = app_context(cx);
    let user = db.fetch_user(42).await;
    Ok(view! { <h1>"Hello, " (user.name)</h1> })
}
```

The type you ask for must be exactly the type you registered. `app_context` panics when no value of that type was registered, which usually means the router setup is missing a line.

A common pattern is to wrap the lookup in a small helper, so the rest of the app calls `db(cx)` instead of repeating the type:

```rust
use topcoat::context::{Cx, app_context};

fn db(cx: &Cx) -> &Database {
    app_context(cx)
}
```

When a value is optional, use `try_app_context::<T>(cx)` instead. It returns `None` when no value of that type was registered:

```rust
use topcoat::context::{Cx, try_app_context};

struct FeatureConfig;

fn feature_config(cx: &Cx) -> Option<&FeatureConfig> {
    try_app_context(cx)
}
```

## Requirements

The value type `T` must be `Any + Send + Sync`. You do not need to write a `'static` bound yourself, because `Any` implies it.

Every request handled by the router borrows the same value, so app context values are read-only. To share state that changes, put the mutable part behind an atomic or a lock:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::context::{Cx, app_context};

struct PageViews(AtomicU64);

fn count_page_view(cx: &Cx) -> u64 {
    app_context::<PageViews>(cx).0.fetch_add(1, Ordering::Relaxed) + 1
}
```

If a handler needs an owned copy of a value, for example to move it into a spawned task, the value should be cheap to clone. Database pools and HTTP clients usually are, because they keep their state behind an `Arc`.
