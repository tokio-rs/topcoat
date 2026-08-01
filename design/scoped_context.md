# Scoped context

This document designs scoped contexts that temporarily add or shadow context values and a unified `Cx` API for mutating request-root values. It also defines how [`#[memoize]`](../crates/topcoat-core/macro/docs/memoize.md) responds when root mutation changes a value that a cached result observed.

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
| Scoped | `cx.with(value)` or `cx.with_values((a, b, ...))` | `request_context(cx)` | That scope and its descendants |

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

# Mutating the request root

`Cx::insert` and `Cx::get_mut` require `&mut Cx`. Layers receive the mutable root context, so they can install request facilities before calling the inner chain:

```rust
#[layer("/")]
async fn cookie_layer(cx: &mut Cx, body: Body, next: Next<'_>) -> Result<Response> {
    cx.insert(CookieJarCell::new());

    let mut response = next.run(cx, body).await?;
    write_cookies(cx, response.headers_mut());
    Ok(response)
}
```

`CxBuilder` is removed. Layers receive `&mut Cx`; routes, pages, and components receive `&Cx`:

```rust
pub trait Layer {
    fn path(&self) -> &Path;

    fn handle<'a>(
        &'a self,
        cx: &'a mut Cx,
        body: Body,
        next: Next<'a>,
    ) -> LayerFuture<'a>;
}

impl<'a> Next<'a> {
    pub fn run(self, cx: &'a mut Cx, body: Body) -> LayerFuture<'a> {
        unimplemented!()
    }
}
```

The router inserts built-in request values into the root `Cx` before invoking the layer chain. A layer may mutate the root before or after `Next::run`. Once the chain reaches a route, the route receives an immutable borrow and may create scoped contexts while rendering.

A scoped context exposes only shared access to `Cx`. The root cannot be mutated while that scoped context, or any other borrow derived from `Cx`, may still be used:

```rust
{
    let section_cx = cx.with(HeadingLevel(2));

    // cx.insert(...) would fail to compile here because section_cx is used below.
    render_security_section(&section_cx).await?;
}

cx.insert(HeadingLevel(3));
```

`insert` adds or replaces a root value and returns the displaced value. `get_mut` returns mutable access to an existing root value:

```rust
assert_eq!(cx.insert(HeadingLevel(1)), None);

*cx.get_mut::<HeadingLevel>().unwrap() = HeadingLevel(2);

assert_eq!(
    request_context::<HeadingLevel>(cx),
    &HeadingLevel(2),
);
assert_eq!(cx.insert(HeadingLevel(3)), Some(HeadingLevel(2)));
```

`insert<T>` and a successful `get_mut<T>` assign a new binding identity before returning. Cached results that observed the previous root `T` no longer match, even if the caller leaves the value returned by `get_mut` unchanged. `get_mut::<T>()` returning `None` changes nothing.

A layer may use a scoped context for work it performs itself, but cannot pass one to `Next::run`, which requires `&mut Cx`.

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

# Binding identity controls reuse

Memoization compares the identity of a context binding, not the value's `PartialEq` result. Two sibling scoped contexts that each contain `DocsVersion::Stable` contain two bindings, so each computes its own variant:

```rust
let first = cx.with(DocsVersion::Stable);
let second = cx.with(DocsVersion::Stable);

let _ = fetch_api_item(&first, "Router").await;  // fetches binding A
let _ = fetch_api_item(&second, "Router").await; // fetches binding B
```

Create one scoped context above sibling calls when they should share the binding. Binding identity works for every `Any + Send + Sync` value without adding an equality bound to `Cx::with`.

Root mutation changes only the affected type's binding identity:

```rust
cx.insert(DocsVersion::Stable); // binding A

let _ = fetch_api_item(cx, "Router").await; // fetches and records A
let _ = fetch_api_item(cx, "Router").await; // reuses A

*cx.get_mut::<DocsVersion>().unwrap() = DocsVersion::Next; // binding B

let _ = fetch_api_item(cx, "Router").await; // A does not match; fetches B

cx.insert(HeadingLevel(2));

let _ = fetch_api_item(cx, "Router").await; // B still matches; reuses
```

Replacing a value with an equal value still creates a new binding identity because context values do not require `PartialEq`. `#[memoize]` cannot detect mutation performed through a `Mutex`, atomic, or another interior-mutability API on `&Cx`. When that state affects a memoized result, mutate the root through `get_mut` before entering immutable rendering, or provide a new scoped binding.

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

`Cx::insert` and `Cx::get_mut` mutate the request root. `Cx::with` and `Cx::with_values` create a child scope and return `CxScope<'_>`. The document calls this value a scoped context; `CxScope` is its concrete implementation type. It dereferences to `Cx` and cannot outlive its parent.

```rust
pub struct Cx {
    // private
}

pub struct CxScope<'cx> {
    // private
}

pub trait ContextValues: private::Sealed {}

impl Cx {
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Any + Send + Sync,
    {
        unimplemented!()
    }

    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
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

`ContextValues` is sealed and implemented for tuples of two through twelve values. `with_values` rejects duplicate types before creating the scope. `with(tuple)` remains available when the tuple itself is one context value.

`Cx` does not implement `Clone`. `CxScope<'_>` implements `Deref<Target = Cx>`, but not `DerefMut`, and exposes no methods that forward mutation to its parent.

# Context storage requirements

The request root stores one value and one `ContextBindingId` for each type. A root binding identity identifies a logical version of the value, not its allocation. `insert<T>` replaces the value, returns the previous value if present, and assigns a fresh identity. A successful `get_mut<T>` assigns a fresh identity before returning `&mut T`. Returning `None` changes nothing.

Each scoped context owns immutable bindings with their own stable identities and links to its parent scope. A scoped binding lives as long as its `CxScope`. Every root version and scoped binding receives a request-unique `ContextBindingId`; IDs are never reused during the request.

Obtaining `&mut Cx` proves that there is no live borrow through a scoped context, context-value reference, memoized-result reference, or memoized future. Root mutation therefore needs no append-only value history or request revision snapshots. The previous root value can be returned immediately; Topcoat does not retain it.

Cached results that recorded the old root identity stop matching after mutation. They may remain in stable cache storage until the request ends; deleting them while `Cx` is mutably borrowed is a possible memory optimization, not part of cache correctness.

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

`try_request_context::<T>` stores a missing read when `T` is absent. App-context reads are not stored because app context cannot change during a request.

This matching rule provides lazy invalidation. Root mutation changes the affected type's binding identity, so old results stop matching without a Salsa dependency graph, request-wide revision, or descendant traversal.

Root mutation could instead increment a request-wide generation while retaining the binding checks required for scopes, but an unrelated mutation would then invalidate every cached result. Updating only the affected root binding's identity preserves precise reuse without another matching mechanism.

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

Root mutation cannot overlap these calls. A memoized future borrows `Cx`, so Rust requires that future and every result reference to end before `insert` or `get_mut`:

```rust
let fetch = fetch_api_item(cx, "Router");

// cx.insert(...) would fail to compile while fetch borrows cx.
let _ = fetch.await;

cx.insert(DocsVersion::Next);
```

Other outcomes follow two rules:

| Event | Result |
| --- | --- |
| The body returns, including `Err` | Store the completed value and wake waiters |
| The body is cancelled or panics | Store nothing and wake a waiter, which checks the cache and may run the body |

Other callers for the same key wait while the body runs. `insert` and `get_mut` cannot run while the memoized body is executing, including across `.await`.

# Non-goals

This proposal preserves `#[memoize]`'s existing non-reentrancy behavior. Recursive memoization and cross-key wait-cycle detection remain outside this design. It does not build an incremental dependency graph; each cached result records only the context lookups needed to validate that result.

# Open questions

- **Layer scoping:** Replacing `CxBuilder` with `&mut Cx` lets every layer mutate the request root, but prevents a layer from passing a read-only `CxScope` to `Next::run`. Is scoping intentionally limited to route and rendering code, or does the layer API need a separate way to scope the inner chain?
- **Lookup names:** Should the chain-aware free functions remain `request_context` and `try_request_context`, or become `scope_context` and `try_scope_context`, or `scoped_context` and `try_scoped_context`? A rename must also decide compatibility aliases and whether `CxTestBuilder::request_context` keeps "request" for request-root registration.
