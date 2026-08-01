# Scoped context

This document designs scoped contexts that temporarily add or shadow context values and a unified `Cx` API for inserting request values. It also defines how [`#[memoize]`](../crates/topcoat-core/macro/docs/memoize.md) responds when an insertion changes a value that a cached result observed.

# Why context needs scoping

Consider a `heading` component that reads the current section's heading level from `Cx` instead of receiving it as a prop:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HeadingLevel(u8);

fn heading_level(cx: &Cx) -> u8 {
    try_request_context::<HeadingLevel>(cx)
        .map(|level| level.0)
        .unwrap_or(1)
}

#[component]
async fn heading(cx: &Cx, text: &str) -> Result {
    let level = heading_level(cx);
    assert!((1..=6).contains(&level));
    let tag = format!("h{level}");

    view! { <(tag)>(text)</(tag)> }
}
```

Pass the scoped context to a plain render helper. The leading `cx =>` applies it to every component in the view:

```rust
async fn render_security_section(cx: &Cx) -> Result {
    view! { cx =>
        heading(text: "Security")
        security_settings()
    }
}

let section_cx = cx.with(HeadingLevel(2));
render_security_section(&section_cx).await?;
```

With this one handoff, `heading` renders `Security` as an `h2`. `security_settings` and its nested components receive the same `&Cx`, so any heading they render also uses level 2.

Rendering through `section_cx` does not change the parent `cx`; the scoped binding is visible only through `section_cx` and its descendants. Scoped context lets one function and the functions it calls use a different value without forwarding an extra parameter or changing the parent scope.

The [URL generation design](https://github.com/tokio-rs/topcoat/pull/225) needs the same behavior when one render uses absolute URLs. `Cx::with` handles both cases without adding a URL-specific flag to `Cx`.

# Context values have three visibility levels

`Cx` provides access to app, request, and scoped context values:

| Level | Added through | Read through | Visible from |
| --- | --- | --- | --- |
| App | `RouterBuilder::app_context(value)` | `app_context(cx)` | Every request handled by the router |
| Request root | `cx.insert(value)` on the root `Cx` | `request_context(cx)` | The root scope and its descendants |
| Scoped | `cx.with(value)`, `cx.with_values((a, b, ...))`, or `scoped_cx.insert(value)` | `request_context(cx)` | That scope and its descendants |

App context is immutable, uses a separate namespace, and cannot be shadowed by request or scoped values. `request_context::<T>` searches scoped bindings from nearest to farthest, then the request root.

# Creating scoped contexts

`Cx::with` takes ownership of one value and returns a scoped context that borrows its parent. Lookup through the returned scope sees the new value:

```rust
let section_cx = cx.with(HeadingLevel(2));

assert_eq!(
    request_context::<HeadingLevel>(&section_cx),
    &HeadingLevel(2),
);
assert_eq!(try_request_context::<HeadingLevel>(cx), None);
```

`Cx::with_values` puts several values in one child scope:

```rust
#[derive(Debug, PartialEq, Eq)]
struct SectionId(&'static str);

let section_cx = cx.with_values((
    HeadingLevel(2),
    SectionId("security"),
));

assert_eq!(request_context::<HeadingLevel>(&section_cx), &HeadingLevel(2));
assert_eq!(request_context::<SectionId>(&section_cx), &SectionId("security"));
```

`with_values((a, b))` stores `a` and `b` as separate typed bindings. `with((a, b))` stores the tuple itself as one binding. Each type may appear once among the values passed to one `with_values` call; duplicate types panic, matching `RouterBuilder::app_context`. Child scopes may still shadow parent bindings.

Dropping a scoped context makes its bindings unreachable through context lookup and leaves the parent unchanged:

```rust
let section_cx = cx.with(HeadingLevel(2));
render_security_section(&section_cx).await?;
drop(section_cx);

assert_eq!(heading_level(cx), 1);
```

# Lookup and shadowing

`request_context::<T>` panics when `T` is absent. `try_request_context::<T>` returns `None`.

Context values use their Rust type as the key. Applications use a newtype or enum instead of a value such as `bool`.

The nearest context value of a type wins. A scoped context can shadow one value while retaining other values from its parent:

```rust
let section_cx = cx.with(HeadingLevel(2));
let nested_section_cx = section_cx.with(HeadingLevel(3));

assert_eq!(
    request_context::<HeadingLevel>(&nested_section_cx),
    &HeadingLevel(3),
);
assert_eq!(
    request_context::<HeadingLevel>(&section_cx),
    &HeadingLevel(2),
);
```

Use `with_values` when several values should share one visibility boundary.

No parent state needs restoring when `nested_section_cx` drops. `section_cx` still resolves `HeadingLevel(2)`.

A scoped context borrows its parent. It cannot outlive the parent or move into detached work that requires `'static`. It can cross an `.await` while the parent remains borrowed.

`with` takes the value by value. `Any` requires the value to be `'static`, and `Send + Sync` preserves the context's thread safety. Shared data can use an `Arc<T>`.

`Cx` remains free of lifetime parameters.

# Inserting into the current scope

`Cx::insert` makes a value current in the scope identified by that `Cx`. Layers use it to install request facilities before calling the inner chain:

```rust
#[layer("/")]
async fn cookie_layer(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let cookies = cx.insert(CookieJarCell::new());

    let mut response = next.run(cx, body).await?;
    write_cookies(cookies, response.headers_mut());
    Ok(response)
}
```

Layers, routes, pages, and components all receive `&Cx`; `CxBuilder` is removed. `Layer::handle` and `Next::run` both take `&Cx`. `Next::run` ties its returned future to that context borrow so a layer can pass either the shared context or a locally created scoped context.

```rust
pub trait Layer {
    fn path(&self) -> &Path;

    fn handle<'a>(
        &'a self,
        cx: &'a Cx,
        body: Body,
        next: Next<'a>,
    ) -> LayerFuture<'a>;
}

impl<'a> Next<'a> {
    pub fn run<'cx>(self, cx: &'cx Cx, body: Body) -> LayerFuture<'cx>
    where
        'a: 'cx,
    {
        unimplemented!()
    }
}
```

The router inserts built-in request values into the root `Cx` before invoking the layer chain. Use `insert` to keep a value visible through the current scope after `Next::run` returns. Use `with` to limit a value to a new child scope and its descendants:

| Operation | Changes | Visible through |
| --- | --- | --- |
| `cx.insert(value)` | The current scope | That `Cx` and descendants without a nearer binding of the same type |
| `cx.with(value)` | A new child scope | The returned scoped context and its descendants |

Inserting a type already present in the current scope replaces the value returned by subsequent ordinary lookups. References acquired before the insertion and memoized calls using an earlier revision continue to see the previous value:

```rust
cx.insert(HeadingLevel(2));
let previous = request_context::<HeadingLevel>(cx);
let section_cx = cx.with(SectionId("security"));

cx.insert(HeadingLevel(3));

assert_eq!(previous, &HeadingLevel(2));
assert_eq!(request_context::<HeadingLevel>(cx), &HeadingLevel(3));
assert_eq!(request_context::<HeadingLevel>(&section_cx), &HeadingLevel(3));
```

`insert` returns a reference to the binding it installed. A layer can retain the exact value it owns across `Next::run`, even if inner code replaces the same type. Append-only storage cannot return the previous `T` by value.

`insert` is available to every function with `&Cx`, not only layers. Concurrent insertions into the same scope are serialized; the last committed binding of each type becomes current. Ordinary lookups use the latest committed revision at the time of lookup.

# Memoization follows observed context values

[`#[memoize]`](../crates/topcoat-core/macro/docs/memoize.md) reuses a result when its arguments match and every request-context lookup made by the function still resolves to the same binding or remains missing.

This function fetches an API item from a remote service. `DocsVersion` selects the documentation subtree, `DocsClient` is shared app context, and `symbol` remains an explicit argument:

```rust
#[derive(Clone, Copy, Debug)]
enum DocsVersion {
    Stable,
    Next,
}

#[memoize]
async fn fetch_api_item(cx: &Cx, symbol: &str) -> Result<ApiItem, FetchError> {
    let version = *request_context::<DocsVersion>(cx);
    let client = app_context::<DocsClient>(cx);

    client.fetch(version, symbol).await
}
```

Concurrent calls with the same symbol and `DocsVersion` binding share one remote fetch:

```rust
let stable_cx = cx.with(DocsVersion::Stable);

let (_main, _sidebar) = tokio::join!(
    fetch_api_item(&stable_cx, "Router"),
    fetch_api_item(&stable_cx, "Router"),
);
// DocsClient::fetch runs once.
```

An unrelated scoped value does not prevent reuse:

```rust
let stable_section_cx = stable_cx.with(HeadingLevel(2));

let _ = fetch_api_item(&stable_section_cx, "Router").await; // reuses
```

A nested scope that shadows `DocsVersion` changes the observed binding, so the function fetches another variant:

```rust
let next_cx = stable_cx.with(DocsVersion::Next);

let _ = fetch_api_item(&next_cx, "Router").await; // fetches next version
```

The same matching rule covers other lookups:

- After `insert(T)`, a call through that scope, or a descendant that inherits the inserted binding, cannot reuse a variant that recorded a different `T` binding or observed that `T` was absent. It may reuse another variant whose recorded lookups still match; otherwise, it recomputes.
- Ancestors, sibling scopes, and descendants with a nearer `T` binding remain unaffected.
- App-context reads add no dependency because scopes cannot shadow them.
- `try_request_context::<T>` records when `T` is missing. A context that supplies `T` cannot reuse that result.

# Binding identity controls reuse

Memoization compares the identity of a context binding, not the value's `PartialEq` result. Two sibling scoped contexts that each contain `DocsVersion::Stable` contain two bindings, so each computes its own variant:

```rust
let first = cx.with(DocsVersion::Stable);
let second = cx.with(DocsVersion::Stable);

let _ = fetch_api_item(&first, "Router").await;  // fetches binding A
let _ = fetch_api_item(&second, "Router").await; // fetches binding B
```

Create one scoped context above sibling calls when they should share the binding. Binding identity works for every `Any + Send + Sync` value without adding an equality bound to `Cx::with`.

Every insertion creates a new binding identity, even when the inserted value compares equal to the previous value. Interior mutation does not create a new binding. If a context value contains a `Mutex`, atomic, or another interior-mutable value, changing its contents does not invalidate a memoized result. Call `insert` or create a scoped context with a new binding when the change must affect memoized work.

# Nested memoized functions carry their dependencies

A memoized function inherits context lookups made by all other memoized functions it calls when those lookups resolve through its input `Cx`. It also inherits lookups missing from both the outer and inner contexts. This applies whether an inner call computes or reads its cache:

```rust
#[memoize]
async fn render_api_item(cx: &Cx, symbol: &str) -> String {
    match fetch_api_item(cx, symbol).await {
        Ok(item) => item.render(),
        Err(error) => render_fetch_error(error),
    }
}
```

`render_api_item` does not call `request_context`, but it records the `DocsVersion` dependency observed by `fetch_api_item`. Stable and next-release contexts therefore cache separate variants of both functions.

When an outer memoized function calls an inner memoized function through a child scope created inside the outer function, the outer result excludes bindings introduced by that child:

```text
outer input:   cx with DocsVersion binding A
outer creates: child_cx with HeadingLevel(2)

inner reads:
|- DocsVersion -> inherited from cx              -> outer records A
|- HeadingLevel -> introduced inside outer       -> outer does not record it
`- SectionId -> missing from cx and child_cx      -> outer records missing
```

A binding created inside the outer function is not an input to that function, so only the inner result records it. The same dependencies propagate when the inner call reuses a cached result instead of running its body.

# Scoped context API

`Cx::insert` changes the current scope. `Cx::with` and `Cx::with_values` create a child scope and return `CxScope<'_>`. The document calls this value a scoped context; `CxScope` is its concrete implementation type. It dereferences to `Cx` and cannot outlive its parent.

```rust
pub struct Cx {
    // private
}

pub struct CxScope<'cx> {
    // private
}

pub trait ContextValues: private::Sealed {}

impl Cx {
    pub fn insert<T>(&self, value: T) -> &T
    where
        T: Any + Send + Sync,
    {
        unimplemented!()
    }

    #[must_use]
    pub fn with<T>(&self, value: T) -> CxScope<'_>
    where
        T: Any + Send + Sync,
    {
        unimplemented!()
    }

    #[must_use]
    pub fn with_values<V>(&self, values: V) -> CxScope<'_>
    where
        V: ContextValues,
    {
        unimplemented!()
    }
}
```

`ContextValues` is sealed and implemented for tuples of two through twelve values. `with_values` rejects duplicate types before creating the scope. `with(tuple)` remains available when the tuple itself is one context value. `CxScope<'_>` implements `Deref<Target = Cx>`.

# Scoped binding requirements

A request owns an append-only binding store and a tree of scopes. Each scope has a stable identity, a stable parent, and a versioned binding history for each type. `insert` appends a value with a fresh `ContextBindingId`, then atomically advances the request revision and makes that binding current.

Append-only storage retains replaced bindings, bindings from dropped scopes, and completed memoized results until the request ends. Validation determines whether a caller can reuse a result; it never deletes or deallocates a stored binding or result.

Lookup can resolve each scope as of any in-flight revision, so replacing the current binding does not discard history needed by a running memoized body.

An insertion affects only the scope on which `insert` is called and descendants that do not shadow that type:

```rust
let sidebar_cx = cx.with(SectionId("sidebar"));
let article_cx = cx.with(SectionId("article"));

sidebar_cx.insert(HeadingLevel(2));

assert_eq!(try_request_context::<HeadingLevel>(cx), None);
assert_eq!(try_request_context::<HeadingLevel>(&article_cx), None);

cx.insert(HeadingLevel(1));

assert_eq!(
    request_context::<HeadingLevel>(cx),
    &HeadingLevel(1),
);
assert_eq!(
    request_context::<HeadingLevel>(&sidebar_cx),
    &HeadingLevel(2),
); // nearest binding wins
assert_eq!(
    request_context::<HeadingLevel>(&article_cx),
    &HeadingLevel(1),
); // inherits the root binding
```

`Cx` does not expose `get_mut`. Calling `insert` on a scope installs a fresh binding there. If that scope already contains the type, subsequent lookups use the new binding while existing references remain valid. Store a type such as `Mutex<T>` when callers must mutate one value in place.

# How cached results match context

Each cached result stores the request-context lookups made while computing it:

```rust
enum ContextRead {
    Present {
        type_id: TypeId,
        binding_id: ContextBindingId,
    },
    Missing {
        type_id: TypeId,
    },
}
```

Before reuse, Topcoat compares every stored read with what the caller's `Cx` resolves:

| Stored read | Caller resolves | Reusable |
| --- | --- | --- |
| Binding `A` | Binding `A` | Yes |
| Binding `A` | Binding `B` or missing | No |
| Missing | Missing | Yes |
| Missing | Any binding | No |

Starting from an empty cache, the `fetch_api_item` example can store different results for callers using different contexts:

```rust
let stable_cx = cx.with(DocsVersion::Stable); // binding A
let next_cx = cx.with(DocsVersion::Next); // binding B

let _ = fetch_api_item(&stable_cx, "Router").await; // fetches and records A
let _ = fetch_api_item(&next_cx, "Router").await; // A does not match; fetches B
let _ = fetch_api_item(&stable_cx, "Router").await; // A still matches; reuses
```

The failed match through `next_cx` does not delete the result for binding `A`. Matching is per caller, so `stable_cx` can still reuse it.

`try_request_context::<T>` stores a missing read when `T` is absent. App-context reads are not stored because app context cannot change during a request. Like Salsa, Topcoat checks recorded inputs before recomputing.

# Concurrent calls for one cache key

Within a request, Topcoat runs one memoized body at a time for each function and combination of non-`cx` arguments. It cannot separate same-key calls by context before the body runs because context reads are discovered while it runs.

Starting from an empty cache, these calls have the same function and `"Router"` argument, even though they use different contexts:

```rust
let stable_cx = cx.with(DocsVersion::Stable); // binding A
let next_cx = cx.with(DocsVersion::Next); // binding B

let (_stable, _next) = tokio::join!(
    fetch_api_item(&stable_cx, "Router"),
    fetch_api_item(&next_cx, "Router"),
);
```

One possible execution is:

| Step | Stable call | Next call | Stored results |
| --- | --- | --- | --- |
| 1 | Runs the body | Waits | None |
| 2 | Stores the stable result, which recorded `A` | Checks the stable result: `A` does not match `B` | Result recording `A` |
| 3 | Done | Runs the body | Result recording `A` |
| 4 | | Stores the next result, which recorded `B` | Results recording `A` and `B` |

Calls with different symbols use different keys and can run together:

```rust
let (_context, _view) = tokio::join!(
    fetch_api_item(&stable_cx, "Cx"),
    fetch_api_item(&stable_cx, "View"),
);
```

Calls to different memoized functions also run independently.

In a separate execution, an outermost memoized call captures the request revision before it checks the cache or waits. An insertion can create a later revision while the body runs:

| Step | Memoized call | Request context |
| --- | --- | --- |
| 1 | Captures revision 7 and starts | `DocsVersion::Stable` at revision 7 |
| 2 | Continues using revision 7 | `insert(DocsVersion::Next)` creates revision 8 |
| 3 | Stores and returns the stable result | Revision 8 remains current |
| 4 | A later call captures revision 8, rejects the stable result, and fetches the next version | Revision 8 remains current |

A waiter keeps the revision it captured before waiting. Nested memoized calls use the outermost call's revision. A child created with `with` or `with_values` sees its own bindings plus ancestor bindings from that revision. Topcoat does not rerun a body when an insertion overlaps it.

Other outcomes follow three rules:

| Event | Result |
| --- | --- |
| The body returns, including `Err` | Store the completed value and wake waiters |
| The body is cancelled or panics | Store nothing and wake a waiter, which checks the cache and may run the body |
| The body calls `Cx::insert` | Panic; `with` and `with_values` remain allowed |

Capturing a request revision and committing an insertion briefly serialize, so the captured revision either includes the insertion or precedes it. Other callers for the same key wait while the body runs, but the body holds no request-context lock while running or awaiting.

# Non-goals

This proposal preserves `#[memoize]`'s existing non-reentrancy behavior. Recursive memoization and cross-key wait-cycle detection remain outside this design. It does not adopt Salsa's durability, value-equality backdating, tracked outputs, or cycle machinery.

# Open questions

- **Tower bridge:** `TowerLayer` currently moves request parts out through the displaced value returned by `CxBuilder::insert`. Append-only `Cx::insert` cannot return ownership of that value, so the bridge needs a separate ownership handoff.
- **Lookup names:** Should the chain-aware free functions remain `request_context` and `try_request_context`, or become `scope_context` and `try_scope_context`, or `scoped_context` and `try_scoped_context`? A rename must also decide compatibility aliases and whether `CxTestBuilder::request_context` keeps "request" for request-root registration.
