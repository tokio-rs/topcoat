# App context

Most apps need values that outlive any single request: a database pool, an HTTP client, a config struct loaded at startup. Topcoat exposes these through **app context**: register a value once on the router, then read it from any handler with `app_context(cx)`.

App context is keyed by Rust type. Each type can be registered at most once, and lookups are typed: ask for `&Database` and you get a `&Database`.

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

The value is stored under its concrete type. Registering two values of the same type panics: wrap them in newtypes if you need more than one of the same underlying type:

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
    view::view,
};

#[page]
async fn user_profile(cx: &Cx) -> Result {
    let db: &Database = app_context(cx);
    let user = db.fetch_user(42).await;
    view! { <h1>"Hello, " (user.name) </h1> }
}
```

The lookup is keyed by `T`'s `TypeId`, so the type you ask for must exactly match the type you registered. Asking for a type that wasn't registered panics: this is a startup-time bug, not a runtime one.

## Requirements

The value type `T` must be `Any + Send + Sync`. There's no `'static` bound to write yourself: `Any` implies it.

App context is shared by reference across every request handled by the router, so values should be cheap to share (typically already wrapped in `Arc` internally, like a database pool or HTTP client) or trivially clonable.

## Sharing services with non-request renderers

[`AppContext`](crate::context::AppContext) is the reusable form of the service
registry. Build one when a renderer such as a mailer, background worker, or CLI
command needs the same services as the router:

```rust
use topcoat::{
    context::{AppContext, RenderContext},
    router::Router,
};
# struct Database;
# impl Database { fn connect() -> Self { Self } }
# struct HttpClient;
# impl HttpClient { fn new() -> Self { Self } }

let services = AppContext::builder()
    .insert(Database::connect())
    .insert(HttpClient::new())
    .build();

let render = RenderContext::new(services.clone());

let router = Router::builder()
    .app_context_map(services)
    .build();
```

`AppContext` is immutable and cheap to clone. `render` and `router` above use
the same registered services. Each renderer creates its own
per-render state while borrowing the same registered services. Call
`app_context_map` before router-local `app_context` registrations.
