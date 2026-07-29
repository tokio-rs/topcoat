# Props

An Inertia prop combines a value source with protocol behavior. Plain values serialize eagerly. Futures resolve only when the current request selects them, and they may borrow the request context for the lifetime of [`Inertia::render`](crate::Inertia::render).

The [`Inertia`](crate::Inertia) and [`Props`](crate::Props) builders provide sugar for common behavior. [`Prop`](crate::Prop) modifiers cover combinations.

```rust
# use topcoat_core::{context::Cx, error::Result};
# use topcoat_inertia::{Inertia, InertiaResponse, defer, lazy, merge};
# async fn load(_: &Cx) -> Result<Vec<u64>> { Ok(vec![1, 2]) }
# async fn example(cx: &Cx) -> Result<InertiaResponse> {
Inertia::new("Dashboard")
    .prop("title", "Dashboard")
    .lazy("user", load(cx))
    .optional("audit", load(cx))
    .prop_with("stats", defer(load(cx)).group("dashboard").rescue())
    .prop_with("posts", merge([1, 2]).once())
    .render(cx)
    .await
# }
```

## Selection

Plain and lazy props are included on an initial page. Optional and deferred futures are not polled on the initial page. Optional props resolve only when a partial reload explicitly requests them. Deferred props publish their path and group in `deferredProps`, and the client adapter requests them after the page mounts.

`always` bypasses partial `only` and `except` filtering. `except` wins when both lists select the same path. If the partial component does not match the page component, Topcoat resolves a full page instead.

## Merge and once behavior

Use `merge`, `append`, `prepend`, or `deep_merge` to control how a new value combines with the value already held by the client. `append_at` and `prepend_at` point at nested collections. `match_on` identifies the nested item key used to update existing collection entries.

Use `once` for data the v3 client may cache across visits. `as_key` separates the client cache key from the prop path, `until` sets an absolute expiry from the render time, and `fresh` forces a new value even when the client reports a cached copy.

The behaviors compose. A prop can be deferred and merged, or merged and cached once. Invalid combinations such as deep merge plus append return a render error with the prop path.

## Nested paths

Dot paths expand into JSON objects and arrays. Numeric segments are array indexes. A later declaration of the same path wins, and a later parent declaration replaces the earlier branch.

```rust
# use topcoat_inertia::Inertia;
let page = Inertia::new("Users/Show")
    .prop("user.name", "Ada")
    .prop("teams.0.name", "Core");
# let _ = page;
```

Empty segments and object/array collisions return a render error.

## Rescue and infinite scroll

`rescue` is available only for deferred props. When that future fails, the page omits its value and lists the path in `rescuedProps` instead of failing the whole response.

[`ScrollMetadata`](crate::ScrollMetadata) carries the query parameter name and previous, current, and next page numbers. The request chooses append or prepend. `wrapper` points merging at a collection nested inside the prop value, such as `feed.data`. A reset request suppresses merge metadata and marks the scroll entry for replacement.

## Shared props

[`InertiaConfig::share_with`](crate::InertiaConfig::share_with) is the normal place for application-wide values and request-borrowing futures. [`share`](crate::share) and [`share_with`](crate::share_with) add owned props from a handler. Configured sharing runs first, request-local sharing follows it, and page props run last.

Top-level shared keys are reported in `sharedProps` so v3 instant visits can retain them. `errors` is always injected as an object and shared independently of application declarations.
