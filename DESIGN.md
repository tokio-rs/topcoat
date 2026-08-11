# Streaming SSR

This document proposes streaming server-side rendering for Topcoat: a way for a page to send meaningful HTML immediately, render skeleton UI in place of slow data, and swap in the real content over the same HTTP response as it becomes ready.

The design rests on one structural rule and two new primitives. The rule splits every component in two: the body runs exactly once per request and does the loading, and the returning `view!` becomes the component's render, which the framework runs again as data arrives. `defer` marks a piece of data as allowed to arrive after the first paint, and `boundary` marks a region of the page as independently swappable. Error handling stays exactly what it is in Topcoat today: plain Rust control flow over plain Rust `Result`s.

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

The root cause is that Suspense resumes a stored continuation. This proposal takes the other path: never resume a continuation mid-expression, run the render again from the top instead.

## Proposal

### The Body and the Render

In a `#[component]`, `#[page]`, or `#[layout]`, the returning expression must be a `view!`, and the macro splits the function there. Everything before it is the body: ordinary async Rust that loads data, checks permissions, and computes values. The body runs exactly once per request. The returning `view!` compiles into the component's render: code that runs once per pass over the body's locals and the component's parameters, as many times as the framework needs. How it is realized is the next section.

The split is what makes repeated rendering safe to build on. Every position in `view!` that accepts user code (interpolations, attribute values, conditions, iterator expressions, props) is compiled inside a closure that is not async, so `.await` there does not compile. Every database call, API request, and expensive computation therefore lives in the body, and the body runs once. A fetch without `#[memoize]` is no longer a hazard, because nothing the user forgets to annotate can end up on the re-render path. The compiler enforces the boundary instead of documentation.

Control flow that must be evaluated on every render lives inside the `view!`, which already supports `if`, `for`, `match`, and `let`. A `view!` in any other position is a value: a token to an anonymous component, described under child content below.

### The Component Future

The persistence story is the one Rust already solved for `async fn`. An async function compiles to a state machine that keeps its locals alive across suspension points, borrows included, verified by the borrow checker and laid out by the compiler. `#[component]` leans on exactly that machinery: the whole component compiles to a single async function, the body followed by a render loop that suspends between passes.

```rust
// The user writes:
#[component]
async fn parent(cx: &Cx) -> Result {
    let title = load_title(cx).await?;
    view! {
        <section>
            child(s: &title)
        </section>
    }
}

// The macro generates, conceptually:
async fn parent(cx: &Cx) -> Result<()> {
    let title = load_title(cx).await?;
    let mut children = ChildTable::new();
    loop {
        let pass = next_pass().await;
        // emit <section>, birth or advance child(cx.clone(), &title), emit </section>
    }
}
```

Nothing is extracted into a stored closure and nothing is moved into a framework table. The body's locals stay where they are, in the suspended function's own frame, and the render loop reads them every pass. Invoking a component inside a `view!` becomes an identity-keyed entry in the parent's child collection: the first pass to reach the invocation creates the child future and runs its body, and every later pass advances the stored future through one more render. Each stored future is boxed together with its own clone of the context handle; `Cx` is a cheap shared handle over the request state, so borrows between frames carry props and locals, never the context.

This is what makes the borrow story trivial. `child(s: &title)` lends a body local to a child that outlives the pass, and it compiles, because a future stored in a local can borrow earlier locals of the same frame; that is what async functions do every day. The rule the borrow checker enforces is the honest one: props may borrow anything that outlives the child, the request, a memoized result, a deferred value, the parent's body locals, while a borrow of a per-pass temporary is rejected at the invocation site with the ordinary lifetime error. Teardown is the ordinary drop order of a stack frame, children before the locals they borrow, so even a capture whose `Drop` impl reads a borrowed prop is sound. There is no arena, no unsafe, and no framework restriction on what props may borrow.

Passes stay cheap because a settled component is a single poll. After the body completes, the only suspension points in the function are pass boundaries, since user code inside `view!` cannot await, so advancing a component runs its whole render synchronously. A component born on a later pass runs its body at that point, awaiting real work, and the pass completes when every birth has settled, which is the concurrent first render the framework already performs.

Errors are future completion. A `?` in a render finishes the component future with the error, the subtree below it is dropped, which is ordinary cancellation with compiler-ordered cleanup, and the enclosing layout is invoked again with the `Err` slot as described under errors. Dropping is also eviction: a component orphaned when a `match` changes arms can be removed from its parent's collection and cleaned up.

Application code sees none of this. The signature stays exactly what the user wrote, and components are only invoked from views, so the generated shape never leaks.

### Child Content

A component that accepts a `child: View` parameter receives the caller's trailing nodes, exactly as today. What changes is what backs the value.

The trailing block compiles to an anonymous component owned by its creator: a future capturing the creator's frame under the same rules as any prop, stored in the creator's child table, keyed by the invocation site. Invocations inside the block live in the block's own table. The creator advances it, the creator's boundary drives its parked births, and the creator's control flow governs its life, exactly as for a named component.

`View` itself is a plain token naming the anonymous component's output slot. It carries no lifetime and no borrow, because the state stays with the owner; passing it to a component is passing a name. Placing it, `(child)` in the receiver's view, emits the slot's marker into the receiver's output at that position. The receiver and the content advance at their own pace and meet only at assembly, the same displacement that decouples every parent from a parked birth. A token may be placed at most once per pass.

Content runs whether or not it is placed. The owner advances it regardless, so a receiver that hides its child this pass keeps the content warm, and placing it on a later pass shows its current state. This preserves the concurrent renderer's behavior today, where the generated invocation joins the receiver's render with the content block and the content executes eagerly whether or not the receiver embeds it; only its bytes are lazy. An unplaced slot is unreachable from the root, so its bytes never travel.

Errors surface at placement. A content block whose render fails leaves the error at its slot; placing the token delivers it, and the receiver propagates it through the default `(child)` placement or catches it the way a layout catches its `slot`. Content that fails and is never placed hands the error back to its owner when the pass seals, so nothing is lost. The surface for catching at a placement, a method on the token or a match form, is a macro design question to settle during implementation.

This also settles what a `view!` is in every position: a token to an anonymous component registered at its creation site. A `view!` bound in the body is created once and placed by embedding the token; one created inside a render is recreated each pass, harmlessly, because identity lives with the site and not the instance.

### The `defer` Primitive

`defer` wraps a future and immediately returns an enum instead of awaiting it:

```rust
pub enum Deferred<T> {
    Pending,
    Ready(T),
}
```

`defer` is called inside `view!`. On the first render that reaches it, it registers the future and returns `Deferred::Pending`. The render matches on the value and produces whatever it wants for the pending case, typically a skeleton:

```rust
use topcoat::{
    Result,
    context::Cx,
    view::{Deferred, component, defer, view},
};

#[component]
async fn drink_grid(cx: &Cx) -> Result {
    view! {
        match defer(cx, drinks(cx)) {
            Deferred::Pending => <div class="grid gap-4 sm:grid-cols-2">
                for _ in 0..6 {
                    <div class="h-32 animate-pulse rounded-lg bg-muted"></div>
                }
            </div>,
            Deferred::Ready(drinks) => <div class="grid gap-4 sm:grid-cols-2">
                for drink in drinks? {
                    drink_card(key: &drink.slug, drink: drink)
                }
            </div>,
        }
    }
}
```

There is no fallback parameter, no lazy closure, and no new control flow construct. `Deferred` is a plain enum handled with a plain `match` inside the `view!`, and both arms are ordinary code with full access to the body's locals and the component's parameters.

`defer` belongs to the render. Calling it in the body would be a bug: the body runs once, so a `Pending` observed there could never be observed again. The API should make that placement a compile error; whether through a context type that only exists inside `view!` or a check in the macro is an implementation question.

The future handed to `defer` is bounded by the request, not the component: it may use `cx` and request-cached data but not the component's locals. Deferred work runs between passes, while the component that registered it is suspended, so it cannot hold borrows into the tree that is waiting on it. The mechanism is a cheap clone. `Cx` is a shared handle over the request state, so `defer` boxes an owned clone of the handle together with the call, in effect `Box::pin(async move { drinks(&cx).await })`, and what the framework stores is a `'static` future in plain owned storage; the compiler builds the small self-referential pair inside the box the same way it builds the component frames. The surface stays `defer(cx, drinks(cx))`, rewritten to thread the owned clone. A future that borrows a body local fails the `'static` bound, which is exactly the rule; deferred work that needs component data takes it owned.

A `defer` call is identified across renders by the identity of the enclosing component combined with the call site, obtained via `#[track_caller]`. The identity system's existing rules apply unchanged: a `defer` reached through an unkeyed repeated invocation has an ambiguous identity and fails with the error message naming the invocation that needs a `key:`. A `defer` call that itself repeats inside a `for` within one `view!` needs its own key; the API should offer a keyed variant for that case.

### Render Passes

When a page render completes and no `defer` was called, nothing changes: the response is built and sent exactly as today. Streaming costs nothing unless a page opts in.

When at least one `defer` returned `Pending`, the framework switches the response into streaming mode:

1. The completed HTML of the first render, skeletons included, is sent as the first chunk, together with the status code and headers that render declared. The connection stays open.
2. The registered futures run. When one or more complete, the framework advances every component through one more render, layouts included, within the same request context. No body runs again. On this pass, `defer` calls whose future completed return `Ready`; calls whose future is still running return `Pending`; new `defer` calls encountered for the first time register their futures.
3. The output of the new pass is diffed against the previous pass (see boundaries below) and the changes are appended to the response stream as swap instructions.
4. Steps 2 and 3 loop. The set of completed futures is snapshotted at the start of each pass, so a single pass sees a consistent world. When a pass encounters no `Pending` and no futures remain in flight, the stream closes.

A `Ready` arm can invoke components that no earlier pass reached. Those components are new, so their bodies run at that point, once, exactly as on a first render, and their `view!`s may register `defer` calls of their own. Sequential loading chains nest naturally: each pass peels one layer.

Advancing every component on every pass sounds wasteful and is the deliberate trade of this design. It is what keeps every pass an ordinary render on an ordinary call stack, and the split is what makes it cheap:

- Render code cannot await, so everything slow lives in a body, and bodies run once. What re-runs is view construction over data that is already loaded, which is fast.
- Passes are bounded by the number of deferred loads, typically one or two beyond the first render.
- If pure rendering cost ever matters, skipping a component whose inputs are unchanged is a natural later addition. Nothing in this design depends on it.

`#[memoize]` keeps the role it has today, deduplicating work that several bodies request within one request, like the current user. It is no longer what makes streaming affordable.

### Errors

This is the payoff of re-rendering. When a deferred future produces a `Result`, the `Ready` arm holds that `Result`, and the `?` in the example above is a real `?` on a real call stack: the render runs as ordinary code yielding a `Result` on every pass, so an error on the fifth pass leaves the component exactly the way an error leaves it on the first. A layout that wants custom UI catches it in its `slot` parameter, the same way it catches an immediate error. There is one error mechanism, and deferring a load changes when its error happens, never where it goes. The code that can observe the error, the form it arrives in, and the order of observation stay the same on every pass: the `defer` site first, then the enclosing layouts from the inside out, then the framework.

Propagation is one of the two moves a `Result` offers, and the other works just as well. `Ready` holds the deferred `Result`, so a component can handle failure at the source with a plain pattern, in place, with full access to the surrounding scope:

```rust
view! {
    match defer(cx, drinks(cx)) {
        Deferred::Pending => <div class="h-32 animate-pulse rounded-lg bg-muted"></div>,
        Deferred::Ready(Ok(drinks)) => <div class="grid gap-4 sm:grid-cols-2">
            for drink in drinks {
                drink_card(key: &drink.slug, drink: drink)
            }
        </div>,
        Deferred::Ready(Err(_)) => <p>"The menu is unavailable right now."</p>,
    }
}
```

Handling an error where it appears or passing it upward is the standard Rust choice, made with the standard constructs, at the place the value exists.

Run-once bodies need one clarification here. A layout's body observes `slot` once, usually as a success, and the content behind a successful `slot` keeps updating across passes without the layout's involvement. The run-once rule is a rule against re-running a body on inputs it has already seen. An error surfacing on a later pass is a `slot` value the layout's body has never seen, so the framework invokes the body again with the `Err`, and the layout catches it in the same `match` it uses today. One detail the prototype surfaced: the framework must remember that a slot failed, or a later pass would recreate the slot and replay the failure; the generated slot handling carries that state.

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

The design leans on one rule that Topcoat already imposes: page renders are side-effect free. Concurrent rendering already forbids components from depending on execution order or communicating through shared state, and prefetching already means a page may render without a user looking at the result. The body/view split narrows where that contract carries weight. Bodies run once and are held to nothing new. The code that runs repeatedly is exactly the code inside `view!`, and its central obligation is enforced by the compiler: it cannot await, so it cannot load, block, or wait.

Boundary diffing adds a softer expectation: renders should be deterministic, because a boundary that renders differently from the same data hashes differently and gets re-sent and re-swapped for nothing. Freshly generated random ids, timestamps rendered mid-page, or iteration over unordered maps cause spurious swaps. The result is correct but wasteful, and a swap replaces DOM, which discards focus, scroll position, and input state inside the region. For now this is a documentation concern: the user should keep boundary content stable. Tooling, such as a dev-mode warning when a boundary's hash changes although no `defer` inside it changed state, can come later.

## Prototype

The model is validated by a hand desugared prototype in [`prototype/streaming-model`](prototype/streaming-model). Every construct the macros would generate is written by hand there: components as async functions with a body and a render loop, children as futures stored in the parent's frame, `defer` handles, and a manual driver that runs passes deterministically under test controlled I/O.

The prototype confirms, with tests: bodies run once while renders run per pass, with no memoization anywhere; child props borrow parent body locals and deferred outputs across passes, through two levels of nesting; a settled tree advances in a single poll per pass; passes observe a consistent snapshot of deferred completions; sequential chains peel one layer per pass; errors travel as future completion, are caught by a layout inline or via a stash while the tree is suspended, and unwind only the failing subtree; eviction is a drop, sound even for captures whose `Drop` impl reads a borrowed prop; and identical renders produce byte identical output, the premise of boundary diffing. The compile time rejections hold as `compile_fail` tests: `.await` in a render position, a prop borrowing a per pass temporary, and a deferred future borrowing a body local.

What it deliberately does not validate: `Send` (the prototype is single threaded), the wire format, and child content, which composes from the validated mechanisms and adds only placement bookkeeping. Its deferred futures, `'static` and carrying a cloned context handle, turned out to match the decided mechanism rather than simplify it; see the `defer` section.

## Open Questions

**Future storage and re-execution.** On every pass, the render runs again and constructs a fresh future for each `defer` it reaches. Two things must hold: the work must not run twice, and the value must be produced again on every later pass, because each pass re-renders the arm that consumes it. One stored output observed several times forces a choice between handing out clones, a shared handle, or a borrow.

The current leaning is an implicit memoize. `defer` owns a request-cached slot keyed by its identity: the first pass registers and spawns the future, the output lands in the request cache, and later passes see `Ready(&T)`, a borrow with the request lifetime. This is how `#[memoize]` already behaves, rewriting a memoized function's return type to `&T`, so the two primitives stay consistent and the lifetime question is already answered: cached values live as long as the request. Implicit storage also keeps `defer` self-contained. Deferring a function that is not memoized is still correct and still runs the work once, because the framework owns the output rather than relying on the user's annotations.

Borrowed output has ergonomic consequences that need prototyping before this is settled. A deferred `Result` wants `as_ref`-style rewriting into `Result<&T, &E>`, which the `MemoizeAsRef` trait already provides for memoized functions, plus an `Error: From<&Error>` conversion so `?` keeps working. That conversion would benefit `as_ref` users independently of streaming. Loops over deferred collections yield references, so the example above would pass `&Drink` to `drink_card`. The exact signature and bounds of `defer` remain open.

The prototype suggests a refinement: `defer` returns a handle, created in the body, holding the output slot in the component's own frame, observed from the render by polling the handle. The slot then needs no identity, no keyed variant, and no request cache; `Ready(&T)` is a plain borrow of frame storage, and the future is constructed exactly once instead of once per pass. This inverts the placement rule above, since creating the handle is body work and observing it is render work. The initial implementation keeps the call inside `view!`, identified by call site; the handle stays on the table as a refinement once the macro exists.

**Tail control flow.** The returning expression must be a `view!`, so a component that switches between whole views writes its `match` inside the macro. The macro could also accept a returning `match` or `if` whose arms are all `view!` blocks and fold it into the render loop. Whether that convenience pulls its weight is undecided.

**The pass protocol.** A suspended future cannot receive per-pass arguments, so pass inputs and the output buffer must reach the render loop through task-scoped state, the way view buffers are scoped today. The prototype validates the rest of the protocol: a parent advances its children by polling them in place, and a birth that parks on I/O composes with sealing a pass. Storing deferred work is settled by the clone mechanism under `defer`: the context never owns futures that borrow it, it owns `'static` futures that each carry their own handle clone, so no self referential storage exists anywhere in the runtime. The task model is decided: everything runs on the request task. The framework spawns nothing; deferred futures and component futures alike are polled by the one task driving the request, and that task is `Send` so it runs on a work-stealing scheduler. Every value a component holds across a pass boundary must therefore be `Send`, in line with the bounds `#[memoize]` already imposes. What remains is diagnostic quality: the error for a non-`Send` local held across passes has to point at the local, not at the generated code.

**Pass scheduling.** Re-rendering once per completed future is wasteful when several complete close together. A short batching window before starting a pass would coalesce them. Whether to batch, and for how long, is undecided.

**Limits.** A page that keeps discovering new `defer` calls streams forever. A cap on passes or a deadline per request, after which pending regions keep their skeletons and the stream closes, is probably wanted.

**Clients without the swap script.** Crawlers and JS-less clients receive the skeleton document and inert templates. The semantics of `defer` permit a degenerate blocking mode: run the passes without flushing, and send only the final render once no futures remain, which is byte-equivalent to not streaming at all. Offering that as a per-request mode, for example for known bots or for tests, is cheap insurance. The same blocking behavior is the natural fallback for contexts that cannot stream, such as views rendered to strings or mail bodies.

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

Step back and every kind of update has collapsed into the same shape. A first load, a deferred piece of data arriving, a client-side navigation, a signal change: each one is "render the page, diff against the client, ship the difference". One wire format, one swap script, one server code path, and one rule for application code, which Topcoat demands already and the body/view split partly enforces at compile time: renders are functions of their inputs, free of side effects. The page becomes a pure function from route and state to HTML, and the boundary diff is what makes calling that function cheap enough to call it for everything.

That is why this design is worth its two primitives. `defer` and `boundary` are small, but they put the framework on a trajectory where interactivity, navigation, and streaming stop being separate features with separate mechanisms and become one mechanism observed at different moments. And if re-execution cost ever becomes measurable along the way, components whose renders are skipped when their inputs are unchanged slot in as a pure optimization, with no change to any semantics above.
