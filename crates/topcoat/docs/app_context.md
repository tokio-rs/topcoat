# App context

App context shares values across requests. Register a value on the router, then borrow it with `app_context(cx)` wherever you have a request context. A database pool is a typical app context value.

Values are identified by their Rust type. The router accepts one value of each type.

## Registering values

Build the router and chain `.app_context(value)` for every value you want to share:

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

Registering two values of the same type panics. Wrap them in distinct types when you need to share both:

```rust
struct PrimaryDb(Database);
struct ReplicaDb(Database);

Router::builder()
    .app_context(PrimaryDb(Database::connect_primary()))
    .app_context(ReplicaDb(Database::connect_replica()))
    .build();
```

## Reading values

Inside any handler that has access to a `Cx`, call `app_context::<T>(cx)` to borrow the registered value:

```rust
use topcoat::{
    context::{Cx, app_context},
    Result,
    router::page,
    view::{View, view},
};

#[page]
async fn user_profile(cx: &Cx) -> Result<impl View> {
    let db: &Database = app_context(cx);
    let user = db.fetch_user(42).await;
    Ok(view! { <h1>"Hello, " (user.name) </h1> })
}
```

The requested type must exactly match the registered type. `app_context` panics if that type was not registered.

When an app context value is intentionally optional, use `try_app_context::<T>(cx)` instead. It returns `None` when the type was not registered:

```rust
use topcoat::context::{Cx, try_app_context};
#
# struct FeatureConfig;

fn feature_config(cx: &Cx) -> Option<&FeatureConfig> {
    try_app_context(cx)
}
```

## Requirements

The value must implement `Any + Send + Sync`. The `Any` bound requires it to be `'static`, so it cannot borrow temporary data.

Requests borrow the registered value. The value does not need to implement `Clone`.
