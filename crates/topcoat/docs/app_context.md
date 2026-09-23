# App context

App context shares values across requests. Register a value on the router, then borrow it with [`app_context`](crate::context::app_context) wherever you have a `&Cx`.

Values are looked up by their Rust type. The router accepts one value of each type.

## Registering values

Call `.app_context(value)` for each value you want to share:

```rust
use topcoat::router::{Router, RouterBuilderDiscoverExt};
# struct Database;
# impl Database { fn connect() -> Self { Self } }
# struct HttpClient;
# impl HttpClient { fn new() -> Self { Self } }

pub fn router() -> Router {
    Router::builder()
        .discover()
        .app_context(Database::connect())
        .app_context(HttpClient::new())
        .build()
}
```

Registering the same type twice on the router panics. Use wrapper types to distinguish values with the same underlying type:

```rust
# use topcoat::router::Router;
# struct Database;
# impl Database {
#     fn connect_primary() -> Self { Self }
#     fn connect_replica() -> Self { Self }
# }
struct PrimaryDb(Database);
struct ReplicaDb(Database);

Router::builder()
    .app_context(PrimaryDb(Database::connect_primary()))
    .app_context(ReplicaDb(Database::connect_replica()))
    .build();
```

## Reading values

Read the registered value by type:

```rust
# struct Database;
# struct User { name: String }
# impl Database {
#     async fn fetch_user(&self, _: u64) -> User { User { name: "Ada".into() } }
# }
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

The requested type must exactly match the registered type. [`app_context`](crate::context::app_context) panics if the type is missing.

For optional values, use [`try_app_context`](crate::context::try_app_context). It returns `None` if the type is missing:

```rust
use topcoat::context::{Cx, try_app_context};
#
# struct FeatureConfig;

fn feature_config(cx: &Cx) -> Option<&FeatureConfig> {
    try_app_context(cx)
}
```

## Requirements

Values must implement `Any + Send + Sync`. They cannot borrow data with a lifetime shorter than `'static`.

Requests borrow the same value, so registration does not require `Clone`. Use synchronization if requests need to mutate it.
