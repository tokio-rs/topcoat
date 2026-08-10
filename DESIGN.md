# Streaming SSR

This document proposes streaming server-side rendering for Topcoat: a way for a page to send meaningful HTML immediately, render skeleton UI in place of slow data, and swap in the real content over the same HTTP response as it becomes ready.

The design rests on two new primitives. `defer` marks a piece of data as allowed to arrive after the first paint, and `boundary` marks a region of the page as independently swappable. Everything else, including error handling, stays exactly what it is in Topcoat today: plain Rust control flow over plain Rust `Result`s.

## Background

### How Topcoat renders today

A `#[page]` is an async function returning `Result<View>`. Layouts wrap pages by path prefix and receive the inner content as `slot: Result<View>`, so a layout can propagate an error with `slot?` or match on it and render custom error UI. Errors are ordinary Rust values: a component deep in the tree fails with `?`, the error bubbles up through the call stack, and whichever layer wants to handle it handles it. There is one error mechanism and it is the language's:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, error::{NotFoundError, RouterErrorExt}, layout, page},
    view::view,
};

#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result {
    // `None` becomes a `NotFoundError` and bubbles out with `?`.
    let post = find_post(cx).await.ok_or_not_found()?;
    view! { <h1>(&post.title)</h1> }
}

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    // The error keeps its type on the way out, so the layout can catch it
    // and replace it with branded UI, or pass anything else along with `?`.
    let content = match slot {
        Err(error) if error.downcast_ref::<NotFoundError>().is_some() => view! {
            (StatusCode::NOT_FOUND)
            <h1>"Nothing here"</h1>
        },
        content => content,
    }?;

    view! {
        <html>
            <body>(content)</body>
        </html>
    }
}
```

Components inside a `view!` render concurrently. Sibling components, loop iterations, and nested components all start at the same time, so a component waiting on a database query does not delay its siblings and request waterfalls do not happen. What the framework does not do today is respond before everything has finished: the whole view is rendered into one buffer and sent as one response. The response is exactly as slow as the slowest component on the page.

Two recent pieces of infrastructure matter for this proposal:

- The component identity system gives every component invocation a stable 128-bit identity derived from the chain of invocation sites leading down to it, disambiguated inside loops by the `key:` argument. The same invocation reached the same way hashes to the same identity on every render.
- `#[memoize]` caches a function's result for the duration of a request, keyed by its arguments, and concurrent callers share one in-flight future. Expensive work called from several places in one request runs once.

### The problem

Some data is slow: a search backend, a third-party API, a heavy aggregate query. Today that slowness sits on the critical path of the first byte. The user stares at a blank tab until the slowest query resolves, even though the shell of the page, the navigation, and most of the content were ready long ago.

The well-known fix is streaming: send the fast parts immediately with skeleton placeholders, keep the connection open, and stream the slow parts in as they resolve. React popularized this model with Suspense, so that is the natural starting point.

## Why a Suspense Port Does Not Fit

React's model: a `<Suspense fallback={...}>` element wraps a subtree. If the subtree suspends, React renders the fallback, streams it, and later streams the real subtree out of order, patching it into place with an inline script. Errors thrown inside the subtree are invisible to ordinary code; they propagate through React internals until an `<ErrorBoundary>` component catches them.

A direct Topcoat translation would look something like this:

```rust
view! {
    suspense(
        fallback: view! { <p>"Loading..."</p> }?,
        lazy: async |cx| {
            let drinks = drinks(cx).await?;
            view! { cx =>
                for drink in drinks {
                    drink_card(key: &drink.slug, drink: drink)
                }
            }
        },
    )
}
```

The `lazy` closure cannot run during the render that builds this view; the whole point is to finish that render without it. So the closure must be stored inside the view and executed later, after the surrounding page function has already returned. That forces it to be `'static`, which means awkward cloning of anything it captures. But the fatal problem is error handling.

When the closure eventually runs and fails, the `?` inside it has nowhere to go. The page function, the layouts, the entire Rust call stack that would normally carry the error upward finished long ago. The error is trapped inside a stored closure with no caller. The only way out is the React way: introduce an `error_boundary` component that the framework consults when a lazy subtree fails.

Now Topcoat has two error mechanisms. Ordinary errors bubble through `?` and are caught by matching on a `Result`; deferred errors teleport to the nearest boundary component and are caught by a completely different construct. Application code has to know which kind of failure it is dealing with, and moving a piece of code into or out of a suspense subtree silently changes how its errors travel. This is precisely the complexity Topcoat's error story exists to avoid.

The root cause is that Suspense resumes a stored continuation. This proposal takes the other path: never store a continuation, re-render instead.

## Proposal

### The `defer` Primitive

`defer` wraps a future and immediately returns an enum instead of awaiting it:

```rust
pub enum Deferred<T> {
    Pending,
    Ready(T),
}
```

On the first render of a request, `defer` registers the future and returns `Deferred::Pending`. The caller matches on the value and renders whatever it wants for the pending case, typically a skeleton:

```rust
use topcoat::{
    Result,
    context::Cx,
    view::{Deferred, component, defer, view},
};

#[component]
async fn drink_grid(cx: &Cx) -> Result {
    match defer(cx, drinks(cx)) {
        Deferred::Pending => view! {
            <div class="grid gap-4 sm:grid-cols-2">
                for _ in 0..6 {
                    <div class="h-32 animate-pulse rounded-lg bg-muted"></div>
                }
            </div>
        },
        Deferred::Ready(drinks) => view! {
            <div class="grid gap-4 sm:grid-cols-2">
                for drink in drinks? {
                    drink_card(key: &drink.slug, drink: drink)
                }
            </div>
        },
    }
}
```

There is no fallback parameter, no lazy closure, and no new control flow construct. `Deferred` is a plain enum handled with a plain `match`, in the component body or inside `view!`, and both arms are ordinary code with full access to the surrounding scope.

A `defer` call is identified across renders by the component identity of the enclosing body combined with the call site, obtained via `#[track_caller]`. The identity system's existing rules apply unchanged: a `defer` reached through an unkeyed repeated invocation has an ambiguous identity and fails with the error message naming the invocation that needs a `key:`. A `defer` call that itself repeats inside a loop within one component body needs its own key; the API should offer a keyed variant for that case.

### Render Passes

When a page render completes and no `defer` was called, nothing changes: the response is built and sent exactly as today. Streaming costs nothing unless a page opts in.

When at least one `defer` returned `Pending`, the framework switches the response into streaming mode:

1. The completed HTML of the first render, skeletons included, is sent as the first chunk, together with the status code and headers that render declared. The connection stays open.
2. The registered futures run. When one or more complete, the framework re-renders the entire page, layouts included, within the same request context. On this pass, `defer` calls whose future completed return `Ready(value)`; calls whose future is still running return `Pending`; new `defer` calls encountered for the first time register their futures.
3. The output of the new pass is diffed against the previous pass (see boundaries below) and the changes are appended to the response stream as swap instructions.
4. Steps 2 and 3 loop. The set of completed futures is snapshotted at the start of each pass, so a single pass sees a consistent world. When a pass encounters no `Pending` and no futures remain in flight, the stream closes.

Because a later pass may call `defer` on data that only became reachable once earlier data arrived, sequential loading chains nest naturally: each pass peels one layer.

Re-rendering the whole page sounds wasteful and is the deliberate trade of this design. It is what keeps the call stack alive, and the cost is small in practice:

- All passes share one request, so everything `#[memoize]`d runs once. The functions worth memoizing, database queries and other I/O, are exactly the expensive parts; the demo's `drinks(cx)` already works this way. What re-runs is view construction, which is fast.
- Passes are bounded by the number of deferred loads, typically one or two beyond the first render.
- If pure rendering cost ever matters, React-style memoized components that skip re-execution when their props are unchanged are a natural later addition. Nothing in this design depends on them.

### Errors

This is the payoff of re-rendering. When a deferred future produces a `Result`, the `Ready` arm holds that `Result`, and the `?` in the example above is a real `?` on a real call stack: the pass that observes the error is a full render of the page, so the error bubbles out of the component, through the page, into the layouts, exactly as it does today. A layout that wants custom UI catches it in its `slot` parameter, the same way it catches an immediate error. There is one error mechanism, and deferring a load does not change how its errors travel.

Two things are different mid-stream, and both are inherent to streaming rather than to this design:

- The status code and headers went out with the first chunk. Status codes and headers declared by later passes are discarded; an error that bubbles to the very top can no longer turn the response into a 500. The framework renders its error view and swaps it in as a whole-page update. Whole-page error swaps are the unhandled fallback, not the intended path; a page that defers fallible work should let a layout catch the error and render something sensible.
- Redirects must keep working after the stream has begun. A redirect error surfacing on a later pass is translated into a swap-stream instruction that makes the client navigate, instead of a `Location` header.

### The `boundary` Primitive

Re-rendering the page on the server is cheap; re-sending the page over the network is not. `boundary` is the opt-in primitive that makes the stream ship only what changed.

A boundary is a component that wraps its children in marker comments:

```rust
view! {
    <h1 class="text-3xl font-bold tracking-tight">"The menu"</h1>

    boundary(
        drink_grid()
    )
}
```

A boundary's identity is its component identity, so the framework can find the same boundary again on the next pass, and the usual `key:` rules cover boundaries in loops. After each pass, the framework hashes the rendered content of every boundary region, with one twist: the region of each nested child boundary is replaced by that child's identity before hashing. A change inside a child therefore changes only the child's hash, not every ancestor's.

Diffing pass N against pass N-1 is then a hash comparison per boundary. Only boundaries whose hash changed are written to the stream; unchanged regions, usually most of the page, are never retransmitted and their DOM is never touched. Structural changes fall out of the same rule: a boundary that appears, disappears, or moves changes its parent's placeholder sequence and thus the parent's hash, so the parent swap carries the new structure. When a parent swaps, its unchanged descendant boundaries could additionally be elided from the payload and preserved in the DOM; that is an optimization, not a requirement.

Boundaries are purely an efficiency feature. A page with `defer` and no `boundary` still streams correctly; the page as a whole acts as the implicit outermost boundary and each pass that changes anything re-sends it entirely. Boundaries can be added afterwards, exactly where the skeletons are, to make the stream surgical.

### Wire Format

The first chunk is a normal HTML document in which each boundary region is delimited by marker comments carrying the boundary's identity hash:

```html
<!--topcoat-boundary 1a2b3c...-->
<div class="grid gap-4 sm:grid-cols-2">...skeleton...</div>
<!--/topcoat-boundary 1a2b3c...-->
```

Every later chunk is appended after `</html>`:

```html
<template data-topcoat-swap="1a2b3c...">
    <div class="grid gap-4 sm:grid-cols-2">...real content...</div>
</template>
```

Appending after `</html>` is deliberate. Topcoat does not know where the user's document ends, and the HTML parser reparents late content into `body`, so this works in every browser and stays within the standard. It also keeps the server-side implementation trivial: passes append, nothing is spliced.

A small swap script shipped with the first chunk watches for arriving templates, locates the matching comment range, replaces the range's contents, and removes the template. Swapped content can contain runtime attributes (`@` handlers, `:` binds, signals), so the swap script must give the runtime a chance to initialize new nodes. Redirects arrive as their own instruction, for example `<template data-topcoat-redirect="/target">`. How the script is packaged, always injected when a response streams versus an explicit component like `topcoat::runtime::script()`, is left open; it is small and independent of the full runtime either way.

One server-side implementation note: response compression must flush at chunk granularity, or buffering defeats the streaming.

## Requirements on Application Code

The design leans on one rule that Topcoat already imposes: page renders are side-effect free. Concurrent rendering already forbids components from depending on execution order or communicating through shared state, and prefetching already means a page may render without a user looking at the result. Re-rendering the page a few times per request is safe under exactly the same contract, so streaming adds no new mental model, only more weight on the existing one.

Boundary diffing adds a softer expectation: renders should be deterministic, because a boundary that renders differently from the same data hashes differently and gets re-sent and re-swapped for nothing. Freshly generated random ids, timestamps rendered mid-page, or iteration over unordered maps cause spurious swaps. The result is correct but wasteful, and a swap replaces DOM, which discards focus, scroll position, and input state inside the region. For now this is a documentation concern: the user should keep boundary content stable. Tooling, such as a dev-mode warning when a boundary's hash changes although no `defer` inside it changed state, can come later.

## Open Questions

**Future storage and re-execution.** On every pass, the code path leading to a `defer` runs again and constructs a fresh future; the framework must not run the work twice, and must produce the value again on each subsequent pass. Candidate answers, not mutually exclusive:

- `defer` requires a `'static + Send` future, spawns it once, stores the output, and hands it back on later passes, which pushes toward `T: Clone` or a shared reference.
- `defer` takes a closure that only runs on the pass that registers it.
- `defer` leans on `#[memoize]`: `defer(cx, drinks(cx))` registers interest on the first pass, and on later passes the freshly built future resolves instantly from the request cache, so the framework never stores an output at all. Memoized functions also already answer the lifetime question, since their results live as long as the request.

The memoize combination is the most attractive because it reuses existing machinery and matches how expensive work should be written anyway, but the exact signature and bounds of `defer` need prototyping.

**Pass scheduling.** Re-rendering once per completed future is wasteful when several complete close together. A short batching window before starting a pass would coalesce them. Whether to batch, and for how long, is undecided.

**Limits.** A page that keeps discovering new `defer` calls streams forever. A cap on passes or a deadline per request, after which pending regions keep their skeletons and the stream closes, is probably wanted.

**Clients without the swap script.** Crawlers and JS-less clients receive the skeleton document and inert templates. The semantics of `defer` permit a degenerate blocking implementation, awaiting the future inline and returning `Ready` on the first pass, which is byte-equivalent to not streaming at all. Offering that as a per-request mode, for example for known bots or for tests, is cheap insurance. The same blocking behavior is the natural fallback for contexts that cannot stream, such as views rendered to strings or mail bodies.

## Why This Pays Off Later

Streaming is the first consumer of the boundary machinery, not its ceiling. What this proposal actually builds is a general operation: render the page, diff its boundary tree against whatever the client already holds, ship only the difference. Between passes of one request, the baseline is the previous pass. But nothing about the diff cares where the baseline comes from, and that one degree of freedom turns the same machinery into the foundation for the two features Topcoat most wants next. Implementing them is out of scope here; designing the diff against an arbitrary baseline instead of hardwiring it to the previous pass is cheap now and is what keeps these doors open.

### Client-Side Navigation

The client knows its current boundary tree, identities and hashes, because the server rendered it. On a client-side navigation, the client sends that tree to the server, tentatively in an `X-Topcoat-Boundaries` header, and the server diffs its very first render of the target page against the client's state instead of a previous pass. Everything the two pages share, the document shell, the navigation, the footer, every layout the routes have in common, hashes identically and never travels. The response carries only the boundaries that actually differ.

This composes with `defer` for free. A navigation response can arrive as a stream like any other: the changed regions come down first as skeletons, then fill in as their data resolves. Navigation to a slow page feels instant, because the instant part really is sent instantly.

What makes this remarkable is what it does not require. This is the experience single-page application frameworks exist to provide, and the standard price is enormous: application logic compiled for the browser, a hydration step, a client-side router, and a second rendering model that the server-rendered one must stay consistent with. Here it falls out of a header and a diff the server already knows how to compute. The server remains the only place rendering happens, the browser holds nothing but the swap script, and a page written for a full document load works for client navigation without a single change. Prefetching gets cheaper for the same reason: a prefetched navigation response is small because it excludes everything the client already has.

### Signals Without Shards

Shards exist because sometimes the markup itself needs the server: fresh search results as the user types. Today that means extracting the markup into a `#[shard]`, a separate server endpoint with its own arguments, its own untrusted-input surface, and code pulled out of the page that owns it.

The refetch model absorbs this. The server can track which signals a page reads during a render. When one of them changes in the browser, the client refetches the page itself, sending the current signal values up in a header. The server prefills the signal reads with the client's state and re-renders the page, which is just an ordinary render of ordinary code. The boundary diff then does what it always does: regions whose output did not depend on the changed signal hash identically and stay untouched, and only the regions that genuinely changed travel back.

The shard's job now emerges from a plain page instead of a dedicated endpoint. No code moves out of the page, no second endpoint is exposed, and the granularity of what updates is not decided upfront by where the shard boundary was drawn; it is discovered per update by the hash diff. Whether this replaces shards or complements them is a later discussion, but the direction is clear: dynamic state re-rendering becomes a property every page has, rather than a construct the user opts into and restructures code around.

### One Model for Everything

Step back and every kind of update has collapsed into the same shape. A first load, a deferred piece of data arriving, a client-side navigation, a signal change: each one is "render the page, diff against the client, ship the difference". One wire format, one swap script, one server code path, and one rule for application code, which Topcoat demands already: renders are functions of their inputs, free of side effects. The page becomes a pure function from route and state to HTML, and the boundary diff is what makes calling that function cheap enough to call it for everything.

That is why this design is worth its two primitives. `defer` and `boundary` are small, but they put the framework on a trajectory where interactivity, navigation, and streaming stop being separate features with separate mechanisms and become one mechanism observed at different moments. And if re-execution cost ever becomes measurable along the way, React-style memoized components that skip an unchanged body slot in as a pure optimization, with no change to any semantics above.
