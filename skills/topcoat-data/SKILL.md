---
name: topcoat-data
description: Connect Topcoat applications to databases and other data services. Use when registering data clients, writing request helpers, loading data in views, implementing mutations or transactions, adding per-request caching, or testing persistence code.
---

# Work with application data

Topcoat does not prescribe a database or ORM. Keep persistence behind ordinary Rust types and functions, then make long-lived clients available through application context.

## Register clients

Create pools and clients at startup and register them on the router:

```rust
let database = Database::connect(&settings.database_url).await?;

let app = Router::new()
    .app_context(database)
    .discover();
```

Retrieve the exact registered type. Newtypes make multiple clients unambiguous.

```rust
use topcoat::context::{Cx, app_context};

fn database(cx: &Cx) -> &Database {
    app_context::<Database>(cx)
}
```

Do not open a pool per request or store request-specific state in application context.

## Put data access in functions

Use ordinary `cx: &Cx` functions for domain queries. Let pages and components load the data they own.

```rust
async fn find_project(cx: &Cx, id: ProjectId) -> Result<Project> {
    use topcoat::router::error::RouterErrorExt;

    Ok(database(cx).find_project(id).await?.ok_or_not_found()?)
}
```

Return domain types rather than exposing database rows throughout the UI. Decide explicitly whether absence is optional or a not-found error.

## Cache within a request

Use `#[memoize]` for repeated, idempotent reads during one request:

```rust
use topcoat::context::{Cx, memoize};

#[memoize]
async fn current_team(cx: &Cx, id: TeamId) -> Result<Team> {
    database(cx).find_team(id).await
}
```

All arguments form the cache key. Concurrent identical calls share the work, and cached values live only for the request. Do not memoize mutations, time-sensitive reads, or functions with relevant inputs outside their arguments.

## Implement writes

Keep mutations in route handlers or procedures:

1. Parse and validate untrusted input.
2. Load the authenticated actor and authorize the operation.
3. Start a transaction when multiple writes must be atomic.
4. Apply domain changes.
5. Commit before rendering or redirecting.

Use redirect-after-post for conventional forms. Procedures and shards must repeat authorization on the server; client visibility is not access control.

Avoid holding transactions across view rendering, remote calls, or other slow work. Watch for N+1 queries when components render in loops; batch or preload where the data layer supports it.

## Choose integrations deliberately

Use the data library already present in the application. Toasty is one option demonstrated by Topcoat examples, not a framework requirement. Follow that library's transaction, migration, and error conventions rather than inventing a Topcoat-specific repository layer.

## Test persistence code

- Substitute fake or in-memory clients through `CxTestBuilder::app_context`.
- Test query helpers separately from rendered HTML.
- Give integration tests isolated database state and deterministic fixtures.
- Cover rollback, conflicts, missing records, and authorization failures.
- Keep migrations and schema compatibility in the data library's own test workflow.
