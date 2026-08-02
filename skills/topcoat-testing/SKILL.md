---
name: topcoat-testing
description: Test Topcoat applications, including views, components, routes, request context, sessions, runtime features, and assets. Use when adding tests, choosing a test boundary, building requests, or debugging application behavior.
---

# Test Topcoat applications

Test public behavior at the smallest stable boundary. Prefer ordinary Rust tests and the framework's public APIs over generated internals.

## Choose the boundary

- Test pure domain functions without Topcoat.
- Render a `View` to test components and server HTML.
- Call `Router::handle` to test routing, layers, cookies, and responses.
- Call procedures or shard routes directly for server behavior.
- Use browser tests only for JavaScript behavior, navigation, or timing-sensitive interactions.

## Build request context

Use `CxTestBuilder` for functions that read application or request context. Register the same concrete types the application registers.

```rust
use topcoat::context::CxTestBuilder;

let cx = CxTestBuilder::new()
    .app_context(FakeDatabase::default())
    .build();

let user = current_user(&cx).await?;
```

Add request parts or request-scoped values only when the code under test needs them. Keep each test's context independent.

## Render views

Render the returned `View` and assert meaningful HTML or text:

```rust
let cx = &CxTestBuilder::new().build();
let html = profile(cx, &user)?.render(cx);

assert!(html.contains("Ada Lovelace"));
```

Prefer focused assertions over whole-document snapshots. Use snapshots when the complete markup is the intentional contract.

## Exercise the router

Build an HTTP request and pass it to the router:

```rust
use topcoat::router::{Body, Router, to_bytes};

let request = http::Request::builder()
    .method("GET")
    .uri("/users/42")
    .body(Body::empty())?;
let response = router.handle(request).await;

assert_eq!(response.status(), http::StatusCode::OK);
let body = to_bytes(response.into_body(), usize::MAX).await?;
assert!(body.starts_with(b"<!doctype html>"));
```

Register routes explicitly when discovery or global registries would make tests interfere. Test status, headers, and body separately.

## Test auth and state

- Use fake or in-memory stores through `.app_context(...)`.
- Carry `Set-Cookie` values into later requests when testing a session lifecycle.
- Cover missing, invalid, expired, rotated, and revoked sessions.
- Test authorization on every procedure and shard that exposes protected data.
- Control clocks, identifiers, and randomness where practical.

## Test runtime behavior

Server tests can verify initial HTML, procedure results, and shard responses. Use a real browser for event handlers, binds, signal updates, cancellation, and stale-response races. Do not infer browser correctness from generated markup alone.

## Keep tests reliable

- Avoid network services and shared mutable state.
- Give each test a fresh router, context, and database state.
- Assert content-hashed asset behavior without hard-coding a hash unless it is the subject of the test.
- Run focused tests while iterating, then the workspace's normal format, lint, test, and documentation checks.
