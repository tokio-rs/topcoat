# Streaming SSR

This document proposes streaming server-side rendering for Topcoat: a way for a page to send meaningful HTML immediately, render skeleton UI in place of slow data, and swap in the real content over the same HTTP response as it becomes ready.

The design rests on two new primitives and one structural change. `defer` marks a piece of data as allowed to arrive after the first paint, and `boundary` marks a region of the page as independently swappable. The structural change is behind the scenes: the future that renders a page stays alive after the first HTML is sent, and the pieces of the page that waited on deferred data re-render themselves in place when it arrives. Nothing else re-renders, and error handling stays exactly what it is in Topcoat today: plain Rust control flow over plain Rust `Result`s.

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

Three existing pieces of infrastructure matter for this proposal:

- The instruction buffer. A `view!` invocation does not build a node tree. The outermost invocation allocates an instruction buffer for the whole render; every `view!` nested inside it, such as a component body, appends an instruction block to that same buffer, and a `View` is a cheap handle to a block. A block embeds a child block by reference, and when the child does not exist yet, the parent embeds a reserved slot that redirects to the child's block once it is filled. This indirection is how a component already renders concurrently with its own children: the parent's output points at the child, it does not contain it.
- The component identity system gives every component invocation a stable 128-bit identity derived from the chain of invocation sites leading down to it, disambiguated inside loops by the `key:` argument. The same invocation reached the same way hashes to the same identity on every render.
- `#[memoize]` caches a function's result for the duration of a request, keyed by its arguments, and concurrent callers share one in-flight future.

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

The root cause is that Suspense detaches a continuation from the call stack that created it. The rest of this proposal is about refusing to do that: the continuation and its stack stay together, kept alive as one value, which is exactly what a Rust future already is.

## Why Not Re-Render From the Root

There is a simpler way to keep a call stack available: re-create it. An earlier revision of this proposal did exactly that. When any deferred future completed, the framework re-invoked the page function, layouts and all, and on the new pass each `defer` whose future had completed returned `Ready`. Errors work perfectly under that model, because the pass that observes an error is a full render with a full call stack.

The problem is everything else that re-runs. Every pass re-invokes every component on the page, and with them every data-loading call, however unrelated to the data that actually arrived. That is only affordable if every expensive function is memoized, which quietly turns `#[memoize]` from an optimization into a correctness requirement, and forgetting it has no visible symptom: the page works, it just silently multiplies database queries and API calls per request.

This proposal keeps the property that made re-rendering attractive, a live call stack for errors to bubble through, and drops the re-execution. After the first render, the only user code that runs again is code whose input actually changed.

## Proposal

### The `defer` Primitive

`defer` wraps a future and immediately returns an enum instead of awaiting it:

```rust
pub enum Deferred<T> {
    Pending,
    Ready(T),
}
```

A `defer` is written inside `view!` as the scrutinee of ordinary control flow, marked with `:`:

```rust
match :defer(drinks(cx)) {
    Deferred::Pending => {
        for _ in 0..6 {
            <div class="h-32 animate-pulse rounded-lg bg-muted"></div>
        }
    }
    Deferred::Ready(drinks) => {
        for drink in drinks? {
            drink_card(key: &drink.slug, drink: drink)
        }
    }
}
```

On the first paint the match renders its `Pending` arm, the skeleton. When the future completes, the match runs again with `Ready`, and its output replaces the skeleton in place. There is no fallback parameter, no lazy closure, and no new control flow construct: `Deferred` is a plain enum, both arms are ordinary code with full access to the surrounding scope, and the `?` on the deferred `Result` is a real `?`.

The `:` marker exists for the macro, not the reader. `match` and `if` in a `view!` accept any plain expression, and reactivity changes how the construct compiles, not just what type it scrutinizes, so the macro needs to be told at the call site. `:` is a placeholder spelling; the syntax is an open question below.

A deferred future is polled once where it is created, so data that is already at hand renders `Ready` on the first paint and never streams. Otherwise the future is owned by the compiled construct, called a reactive node below, and fires exactly once, when it moves from `Pending` to `Ready`. Because the node owns its future, the questions that dominated the re-render design do not arise: there is no future to store across passes, no call-site identity to reconnect it by, no keyed variant of `defer`, and no reliance on `#[memoize]` to absorb re-execution.

The one restriction is placement: `defer` must be consumed inside `view!`, through a `:`-marked construct. A `Deferred` matched in plain component-body code has no way to re-run and would render its skeleton forever. The framework should diagnose this, at compile time where the macro can see it and with a debug-mode runtime warning for a `Deferred` that is never consumed reactively.

### Reactive Views

When a deferred future completes, something must re-run the `match`, the arms must still see every variable they close over, and the new output must land in the page without anything around it re-rendering.

Storing the arms in the `View` as a closure cannot work: a `View` outlives the function that built it, so the closure would need to be `'static` and could not borrow the component's locals. That is the Suspense trap again. The design inverts the ownership instead. The `View` stays what it is today, a cheap, cloneable, inert handle into the instruction buffer. The re-run code stays inside the future that is executing the component body, and that future does not return when the view is built.

The rest of this section walks one component through the expansion. It has an interpolation, a child component, and one deferred load:

```rust
#[component]
async fn profile(cx: &Cx) -> Result {
    let user = current_user(cx).await?;

    view! {
        <h1>(&user.name)</h1>
        avatar(user: &user)

        match :defer(orders(cx, &user)) {
            Deferred::Pending => {
                <p class="skeleton">"Loading orders..."</p>
            }
            Deferred::Ready(orders) => {
                <ul>
                    for order in orders? {
                        <li>(&order.title)</li>
                    }
                </ul>
            }
        }
    }
}
```

Today, `view!` expands to three phases: a hoist that evaluates every expression in source order and binds component render futures, a `try_join!` that awaits the components concurrently, and a synchronous burst that lays down the view's instruction block. The new expansion keeps all three and adds slots plus a refresh registration. Simplified:

```rust
// Simplified: what the `view!` in `profile` expands to.
{
    // Hoist: evaluate expressions in source order, as today.
    let __expr0 = &user.name;

    // A component invocation: reserve a slot and start the child. The
    // child fills the slot when its render phase finishes, then stays
    // live in `__refresh` for as long as it has pending work of its own.
    let (__child0, __child0_slot) = internal::reserve();
    let __props0 = avatar::props_builder().user(&user).build();
    __refresh.adopt(avatar::render(__cx, __props0, __child0_slot));

    // A reactive node: a reserved slot, the deferred future, and the
    // arms as a closure that can run for either state.
    let (__node0, __node0_slot) = internal::reserve();
    let __node0_future = orders(cx, &user);
    let __node0_arms = async |__state: Deferred<_>| {
        Ok(match __state {
            Deferred::Pending => internal::block(__cx, |__b| {
                __b.markup(&"<p class=\"skeleton\">Loading orders...</p>");
            }),
            Deferred::Ready(orders) => {
                // An arm is a nested view scope with its own hoist, join,
                // and burst; it may start components and register nodes.
                let __orders = orders?;
                internal::block(__cx, |__b| {
                    __b.markup(&"<ul>");
                    for order in __orders {
                        __b.markup(&"<li>");
                        __b.node(&order.title);
                        __b.markup(&"</li>");
                    }
                    __b.markup(&"</ul>");
                })
            }
        })
    };

    // First evaluation. (A real expansion polls `__node0_future` once
    // here, so data already at hand renders `Ready` on the first paint
    // and registers no refresh.)
    __node0_slot.fill(__node0_arms(Deferred::Pending).await?);

    // The node's refresh: await the data, re-run the match, refill the
    // slot. Pushed, not run: `__refresh` polls it from now on, so the
    // deferred future makes progress while the rest of the page renders.
    __refresh.push(async move {
        let __output = __node0_future.await;
        __node0_slot.refill(__node0_arms(Deferred::Ready(__output)).await?);
        Ok(())
    });

    // Join: wait for every child started above to hand over its view.
    // This replaces today's `try_join!`; refresh work stays live.
    __refresh.barrier().await?;

    // Burst: lay down this view's instruction block, as today.
    internal::block(__cx, |__b| {
        __b.markup(&"<h1>");
        __b.node(__expr0);
        __b.markup(&"</h1>");
        __b.view(__child0);
        __b.view(__node0);
    })
}
```

Three things changed against today's expansion. The child component is not awaited to completion: it fills a reserved slot when its render phase ends, and the `try_join!` became a barrier that waits only for those handovers. The `match` became a reactive node whose slot, future, and arms are plain local values. And the refresh, the only code that will ever run again, is registered instead of executed. Nothing is `'static`: `__node0_future` and `__node0_arms` both borrow `user`, and a second `defer` borrowing `user` would sit right next to the first.

`__refresh` is declared by the `#[component]` expansion, which is where the future learns to outlive the view it produces:

```rust
// Simplified: the future `#[component]` generates for `profile`.
fn render<'cx>(
    cx: &'cx Cx,
    props: ProfileProps<'cx>,
    __handover: ViewSlot,
) -> impl Future<Output = Result<()>> + Send + 'cx {
    async move {
        // Collects the body's live work: adopted children and pushed
        // refreshes. `view!` expansions in the body push into it.
        let mut __refresh = RefreshSet::new();

        // The body, unchanged. A `?` here fails before the yield; the
        // parent's barrier sees the error instead of a handover.
        let user = current_user(cx).await?;
        let __view = { /* the `view!` expansion above */ };

        // The yield: hand the finished view to the parent, then keep
        // going. This is the line where today's generated code returns.
        __handover.fill(__view);

        // The refresh phase: drive children and reactive nodes until
        // none have work left. With nothing adopted or pushed, this
        // completes immediately and the whole future was today's
        // behavior. `user` is alive across this await; that is the
        // point of not returning.
        __refresh.run().await
    }
}
```

The view travels through the handover slot, not the return value; the future's output is the component's terminal status, which is how error transitions bubble. The future completes at quiescence, so a page without `defer` completes on its first pass through and streaming costs nothing. Until then, wakers and joins do all the signaling: a completed deferred future wakes the task, the poll descends through the nested `RefreshSet`s to the node that woke, and the refill marks the render changed.

What runs when a node fires is the arm closure and nothing else. Every enclosing component embedded `__node0` by slot reference, so the refill changes what all of them render without a line of their code executing. An arm that invokes components gives them the full treatment, adoption included, so a `defer` revealed by another `defer` chains naturally while unrelated regions never wait on one another.

Refilling a slot drops the subtree it replaced, and dropping a future is cancellation in Rust: a skeleton with pending work of its own, or a subtree displaced by an error, stops loading the moment it leaves the page.

### The Render Lifecycle

The router composes the page and its layouts into one live render, the same call chain it builds today, and drives it:

```rust
// Simplified: the router driving a streaming response.
let mut render = pin!(compose(layouts, page, cx));

// First paint: the root hands over the document when its render phase
// finishes. Deferred futures have been running since they were created,
// so slow queries overlap the first paint instead of starting after it.
let first = render.first_view().await?;
send_chunk(first.html, first.status_code, first.headers);

// The render stays alive until it is quiescent. Each change pulse means
// nodes fired and refilled slots; re-executing the instruction buffer
// is framework code interpreting instructions, no user code.
while let Some(_changed) = render.next_change().await? {
    let html = render.execute_buffer();
    send_chunk(diff_boundaries(&mut baseline, &html));
}

// The render future completed: everything resolved and shipped.
```

If the render is already complete at first paint, no reactive node registered and the response ends with the first chunk. Fires that land in the same poll cycle coalesce into one chunk for free. The instruction buffer, owned today by the returned view, stays with the live render for the duration of the response, since refills keep writing to it.

Contexts that cannot stream need no second implementation: awaiting the render future to completion instead of taking the first view produces the final document in one piece, every arm settled. That is the natural mode for known crawlers and JS-less clients, for tests, and for renders that are not HTTP responses at all, such as mail bodies. It is byte-equivalent to the document a streaming client converges to.

### Errors

No signatures change. Components and pages return `Result`, layouts receive `slot: Result<View>`, and there is no second view type carrying error state: reactivity lives in the render future, not in the values it passes around.

On the first render, errors travel exactly as today. The render phase is a live call stack; in the expansion above, a failure is the `?` before the yield, and the parent's barrier observes it instead of a handover.

A transition is an error after the first paint: `orders?` fails when the `Ready` arm runs. In the expansion, that is the `?` inside the pushed refresh future. It makes the node's refresh fail, so the component's `RefreshSet::run` produces the error, so the component's own future produces it, and so on up the join tree that mirrors the call chain. A component that invoked the failing child as a plain call passes the error along without any of its code re-running, matching the implicit `?` of a first-render invocation.

The catch points are layouts, as today. The router backs each layout's slot with a reserved slot of its own, so when a transition reaches a layout, the router re-invokes the layout function with `Err(error)` as its slot, the same call it makes today when a page fails outright. Whatever the layout renders, branded error UI or a rethrow to the next layout out, replaces that layout's region, and the displaced subtree's futures are dropped, cancelling its remaining loads. A re-invoked layout re-runs its own data loading; that is the error path and the cost is accepted. If no layout catches, the framework's error view swaps in as a whole-page update.

Nothing forces an error into this machinery: an arm that wants local error UI matches on the `Result` instead of applying `?`, and the failure never leaves the component.

Two things are different mid-stream, and both are inherent to streaming rather than to this design:

- The status code and headers went out with the first chunk. Status codes and headers declared by refreshed content are discarded; an error that bubbles to the very top can no longer turn the response into a 500.
- Redirects must keep working after the stream has begun. A redirect error surfacing after the first chunk is translated into a swap-stream instruction that makes the client navigate, instead of a `Location` header.

A later refinement suggests itself: compiling a `match` on a component invocation's `Result` inside `view!` into a reactive node would give component-level catches whose re-run is the size of an arm, instead of a layout's whole region. Nothing in the design blocks it; it is left out of the first cut.

### The `boundary` Primitive

Re-rendering a slot on the server is cheap; re-sending the page over the network is not. `boundary` is the opt-in primitive that makes the stream ship only what changed.

A boundary is a component that wraps its children in marker comments:

```rust
view! {
    <h1 class="text-3xl font-bold tracking-tight">"The menu"</h1>

    boundary(
        drink_grid()
    )
}
```

A boundary's identity is its component identity, so the client can name the same region across chunks, and the usual `key:` rules cover boundaries in loops. After each change, the framework hashes the rendered content of every boundary region, with one twist: the region of each nested child boundary is replaced by that child's identity before hashing. A change inside a child therefore changes only the child's hash, not every ancestor's.

Diffing the new document against the previous one is then a hash comparison per boundary. Only boundaries whose hash changed are written to the stream; unchanged regions, usually most of the page, are never retransmitted and their DOM is never touched. Structural changes fall out of the same rule: a boundary that appears, disappears, or moves changes its parent's placeholder sequence and thus the parent's hash, so the parent swap carries the new structure. The live render also knows exactly which slots refilled, so hashing can skip boundaries containing no changed slot; that, like eliding unchanged descendants from a parent swap, is an optimization rather than a requirement.

Boundaries are purely an efficiency feature. A page with `defer` and no `boundary` still streams correctly; the page as a whole acts as the implicit outermost boundary and any change re-sends it entirely. Boundaries can be added afterwards, exactly where the skeletons are, to make the stream surgical.

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

Appending after `</html>` is deliberate. Topcoat does not know where the user's document ends, and the HTML parser reparents late content into `body`, so this works in every browser and stays within the standard. It also keeps the server-side implementation trivial: chunks append, nothing is spliced.

A small swap script shipped with the first chunk watches for arriving templates, locates the matching comment range, replaces the range's contents, and removes the template. Swapped content can contain runtime attributes (`@` handlers, `:` binds, signals), so the swap script must give the runtime a chance to initialize new nodes. Redirects arrive as their own instruction, for example `<template data-topcoat-redirect="/target">`. How the script is packaged, always injected when a response streams versus an explicit component like `topcoat::runtime::script()`, is left open; it is small and independent of the full runtime either way.

One server-side implementation note: response compression must flush at chunk granularity, or buffering defeats the streaming.

## Requirements on Application Code

The design leans on one rule that Topcoat already imposes: page renders are side-effect free. Concurrent rendering already forbids components from depending on execution order or communicating through shared state, and prefetching already means a page may render without a user looking at the result. This proposal adds only a time dimension to the same contract: a `:`-marked arm may execute well after the surrounding body finished, and a pending arm's output is discarded when the ready arm replaces it. Code that treats rendering as a pure function of its inputs does not notice. Notably, the contract here is lighter than under the re-render design, which re-executed the entire page and made the rule load-bearing for every function on it.

Boundary diffing adds a softer expectation: renders should be deterministic, because a boundary that renders differently from the same data hashes differently and gets re-sent and re-swapped for nothing. Freshly generated random ids, timestamps rendered mid-page, or iteration over unordered maps cause spurious swaps. The result is correct but wasteful, and a swap replaces DOM, which discards focus, scroll position, and input state inside the region. For now this is a documentation concern: the user should keep boundary content stable. Tooling, such as a dev-mode warning when a boundary's hash changes although no slot inside it refilled, can come later.

## Open Questions

**Reactive syntax.** `:` is a placeholder. To settle: the final spelling; which constructs accept a reactive scrutinee (`match` first, probably `if let`); and whether a reactive expression in node position, for a deferred fragment of text, is worth having. Related: how firmly the misuse cases can be diagnosed, a `Deferred` consumed without a marker or created outside `view!`.

**Threading the refresh set through the macros.** The expansion above hand-waves how `view!` reaches the `__refresh` that `#[component]` declared. The futures involved borrow the component's locals, so they cannot travel through any `'static` registry; a scoped collector local works if the epilogue consumes it, so the borrow checker accepts borrows of locals declared after it, but the pattern needs prototyping. So do the `barrier` semantics and the changed `Component::render` contract, yield-then-continue instead of return-once. For `view!` used outside a `#[component]`/`#[page]`/`#[layout]` transform, reactive nodes should be a compile error, and component invocations should keep completion semantics: the expression awaits the subtree until quiescent, which is exactly the blocking mode above.

**Views that never join the page.** A body can build a view and discard it, or build two and use one. Reactive nodes inside a discarded view must not hold the stream open or load data for invisible content. Candidate: a node stays inert until the renderer first visits its slot, only live nodes are serviced, and inert nodes are dropped when the component's refresh phase otherwise completes.

**Batching.** Completions that arrive in one poll cycle already coalesce into one chunk. Whether to add a short window that also coalesces near-simultaneous completions across wakes is undecided.

**Limits.** A deadline per request is probably wanted: when it expires, the framework stops polling the render, the stream closes, and pending regions keep their skeletons. Dropping the render future cancels all outstanding work, so enforcement is one drop.

**Buffer growth.** Every refill appends new blocks and orphans the ones they replace. For streaming this is bounded by the number of fires, but if the live render later services long-lived updates, compaction or block reuse becomes worth designing.

## Why This Pays Off Later

Streaming is the first consumer of the boundary machinery, not its ceiling. What this proposal actually builds is a general operation: render the page, diff its boundary tree against whatever the client already holds, ship only the difference. Within one request, the baseline is what the response has already carried. But nothing about the diff cares where the baseline comes from, and that one degree of freedom turns the same machinery into the foundation for the two features Topcoat most wants next. Implementing them is out of scope here; designing the diff against an arbitrary baseline instead of hardwiring it to the previous chunk is cheap now and is what keeps these doors open.

### Client-Side Navigation

The client knows its current boundary tree, identities and hashes, because the server rendered it. On a client-side navigation, the client sends that tree to the server, tentatively in an `X-Topcoat-Boundaries` header, and the server diffs its very first render of the target page against the client's state instead of a previous chunk. Everything the two pages share, the document shell, the navigation, the footer, every layout the routes have in common, hashes identically and never travels. The response carries only the boundaries that actually differ.

This composes with `defer` for free. A navigation response can arrive as a stream like any other: the changed regions come down first as skeletons, then fill in as their data resolves. Navigation to a slow page feels instant, because the instant part really is sent instantly.

What makes this remarkable is what it does not require. This is the experience single-page application frameworks exist to provide, and the standard price is enormous: application logic compiled for the browser, a hydration step, a client-side router, and a second rendering model that the server-rendered one must stay consistent with. Here it falls out of a header and a diff the server already knows how to compute. The server remains the only place rendering happens, the browser holds nothing but the swap script, and a page written for a full document load works for client navigation without a single change. Prefetching gets cheaper for the same reason: a prefetched navigation response is small because it excludes everything the client already has.

### Signals Without Shards

Shards exist because sometimes the markup itself needs the server: fresh search results as the user types. Today that means extracting the markup into a `#[shard]`, a separate server endpoint with its own arguments, its own untrusted-input surface, and code pulled out of the page that owns it.

The refetch model absorbs this. The server can track which signals a page reads during a render. When one of them changes in the browser, the client refetches the page itself, sending the current signal values up in a header. The server prefills the signal reads with the client's state and re-renders the page, which is just an ordinary render of ordinary code. The boundary diff then does what it always does: regions whose output did not depend on the changed signal hash identically and stay untouched, and only the regions that genuinely changed travel back.

The live render adds a second route to the same destination. A signal read inside `view!` is shaped exactly like `defer`: a reactive leaf whose change re-runs the arms that read it. Over a connection that keeps the render alive, a signal change would re-run only those arms, with no refetch at all. Whether refetch or a live connection fits better, and where shards end up, are later discussions; both ride the same boundary diff and the same reactive nodes.

### One Model for Everything

Step back and every kind of update has collapsed into the same shape. A first load, a deferred piece of data arriving, a client-side navigation, a signal change: each one is "render, diff against the client, ship the difference". One wire format, one swap script, one server code path, and one rule for application code, which Topcoat demands already: renders are functions of their inputs, free of side effects. The page becomes a pure function from route and state to HTML, and what makes calling it cheap is not memoizing everything but the live render itself, which re-runs only the code whose inputs changed.

That is why this design is worth its cost. `defer` and `boundary` are small on the surface, the live render is real machinery underneath, and together they put the framework on a trajectory where streaming, navigation, and interactivity stop being separate features with separate mechanisms and become one mechanism observed at different moments.
