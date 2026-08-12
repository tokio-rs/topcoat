# Reactive Server Rendering

This document proposes a reactive rendering system for Topcoat. Today a page renders exactly once: the whole view is built into one buffer and sent when the slowest component finishes. Under this proposal the render is alive. It produces the page, keeps running, and re-renders small marked regions in place when the values they consume change. Everything outside those regions still renders exactly once, and error handling stays what it is today: plain Rust control flow over plain Rust `Result`s.

The first feature built on the system is streaming server-side rendering: a page sends meaningful HTML immediately, renders skeleton UI in place of slow data, and swaps in the real content over the same HTTP response as it becomes ready.

The system is three building blocks, and the proposal introduces them in order, smallest first:

- A reactive expression is a value with a state to render now and possibly new states later. `defer`, which wraps a slow future instead of awaiting it, is the first one.
- A `live` construct is view control flow, such as `live match`, that consumes a reactive expression and re-renders its arms in place when the state changes. Nothing else re-renders.
- A `boundary` marks a region of the page as independently swappable, so an update sends only the regions that actually changed.

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

- The instruction buffer. `view!` does not build a node tree. The outermost invocation allocates an instruction buffer for the whole render, every nested `view!` appends an instruction block to it, and a `View` is a cheap handle to a block. A block embeds a child block by reference; when the child does not exist yet, the parent embeds a reserved slot that is filled in later. This is how a component already renders concurrently with its own children: the parent's output points at the child, it does not contain it.
- The component identity system gives every component invocation a stable 128-bit identity derived from the chain of invocation sites leading down to it, disambiguated inside loops by the `key:` argument. The same invocation reached the same way hashes to the same identity on every render.
- `#[memoize]` caches a function's result for the duration of a request, keyed by its arguments, and concurrent callers share one in-flight future.

### The problem

Some data is slow: a search backend, a third-party API, a heavy aggregate query. Today that slowness sits on the critical path of the first byte. The user stares at a blank tab until the slowest query resolves, even though the shell of the page, the navigation, and most of the content were ready long ago.

The well-known fix is streaming: send the fast parts immediately with skeleton placeholders, keep the connection open, and stream the slow parts in as they resolve. React popularized this model with Suspense. This proposal reaches the same experience by a different route: it keeps the render alive and re-renders marked regions in place. The comparison with a direct Suspense port, and with re-rendering the page from the root, comes after the proposal, in the alternatives section.

## Proposal

A page opts into streaming by marking a slow load with `defer` and rendering something in its place; the framework sends the page immediately, keeps the render alive, and swaps the real content in when it arrives. This section builds the model up one piece at a time: the two constructs application code touches, then how reactive values behave as ordinary Rust values, how regions nest and compose, how errors travel, and finally how updates reach the browser. The machinery underneath comes later, in the implementation section.

**A note before the examples: the syntax is not final, and it is not what this proposal is about.** The examples mark a reactive construct with the `live` keyword, as in `live match`; treat it as working syntax. All the design requires is that some visible marker tells a reactive construct apart from a plain one, because the two compile differently. The spelling can be settled last; read the examples for the semantics.

### `defer`: Data That Arrives Later

The first building block is `defer`, a plain function that wraps a future instead of awaiting it:

```rust
pub fn defer<F: Future>(cx: &Cx, future: F) -> Defer<F>
```

Awaiting a slow query stops the component until the data arrives. `defer` returns immediately instead, with a `Defer`: a value that stands for the data before it exists. Inside a view the macro supplies the `cx` argument, the same wiring a component invocation gets, so a view writes `defer(drinks(cx))`; outside a view, `cx` is passed explicitly, as the signature shows. At any moment it is in one of two states, described by a plain enum:

```rust
pub enum Deferred<T> {
    Pending,
    Ready(T),
}
```

A `Defer` is the first example of a reactive expression: a value with a state to render now, and possibly new states later. For `Defer` the states are exactly two: `Pending` now, and `Ready` once, when the future completes. The change happens at most once, and the value is inert afterwards: content never re-renders once it is on the page. On its own a `Defer` renders nothing; a view has to consume it.

### `live`: Control Flow That Re-Renders

The second building block is the `live` construct: view control flow that consumes a reactive expression and renders its current state. Most uses create and consume in one place:

```rust
use topcoat::{
    Result,
    context::Cx,
    view::{Deferred, component, view},
};

#[component]
async fn drink_grid(cx: &Cx) -> Result {
    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            live match defer(drinks(cx)) {
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
        </div>
    }
}
```

The first render sees `Pending` and renders the skeleton grid. When `drinks(cx)` completes, the `Defer` changes state, the match runs its arms again with `Ready`, and the new output replaces the skeleton. Nothing else on the page re-renders, and no other code runs again. That in-place replacement is the whole reactive model; everything else in this proposal is this one mechanism composed, made cheap on the wire, or applied to errors.

Both arms are ordinary code with full access to the surrounding scope, and the `?` on the deferred `Result` is a real `?` that bubbles like any other.

The value can also be created earlier and consumed where it is rendered; outside a view there is no macro to supply `cx`, so it appears explicitly:

```rust
let drinks = defer(cx, drinks(cx));

view! {
    <h1>"The menu"</h1>

    live match drinks {
        // the same arms as above
    }
}
```

A `Defer` follows normal Rust rules. It is a value: create it anywhere, name it, or pass it as a prop. The `match` moves it, so each `Defer` is consumed exactly once; the borrow checker enforces this. And like every Rust future it is lazy: it runs only once a view consumes it, so deferring a query earlier in the body does not start it earlier. A `Defer` dropped without being consumed never runs at all; the type is `#[must_use]`, so the compiler catches the usual mistake of creating one and not using it.

Data that is already at hand, a cache hit or a memoized call another component already made, reports `Ready` on the first poll: the ready arm renders on the first paint and nothing streams. `defer` on fast data costs nothing but a skeleton that is never shown, so it is safe to use whenever data might be slow.

The `live` keyword is there for the macro. A plain `match` and a `live match` compile differently, and the macro cannot tell them apart from the expression alone, so the keyword does it. The value a `live` construct consumes must implement a small contract, described in the implementation section; `Defer` is its first implementation.

### The Render Stays Alive

The example raises the question the design turns on. By the time `drinks(cx)` completes, `drink_grid` has returned its view, and the page function that called it may have finished long ago. What runs the `Ready` arm?

The answer is the one structural change in this proposal. Today the future that renders a page runs until the view is built, returns it, and is dropped. Under this proposal the same future hands its view over and keeps running until nothing inside it can change anymore. The component's locals stay alive on the future's frame, the deferred future makes progress inside it, and when the data arrives the render wakes up, re-runs the arms of the one construct that owns the `Defer`, and splices the new output into the already-rendered page. Every enclosing component embedded that region by reference, so the page's output changes without any of the page's code re-running.

Over the wire this becomes streaming. The first chunk is the page with skeletons where pending arms rendered, sent as soon as the page has rendered around them. The connection stays open; each re-render becomes a later chunk on the same response, and a small script swaps the new content into place. When nothing deferred remains, the stream closes. A page with no `defer` finishes its render in one pass and is sent as one response, exactly as today. What a chunk carries is settled at the end of the proposal: the whole page by default, and only the regions that actually changed once `boundary`, the third building block, is introduced. Until then, the examples only need "the arms re-run and the region updates in place".

### Deferred Values Are Ordinary Values

The future handed to `defer`, and the arms that consume its output, borrow like any other Rust code:

```rust
#[component]
async fn profile(cx: &Cx) -> Result {
    let user = current_user(cx).await?;

    view! {
        <h1>(&user.name)</h1>

        live match defer(orders(cx, &user)) {
            Deferred::Pending => order_skeleton(),
            Deferred::Ready(orders) => order_list(orders: orders?),
        }

        live match defer(recommendations(cx, &user)) {
            Deferred::Pending => reco_skeleton(),
            Deferred::Ready(recos) => reco_list(recos: recos?),
        }
    }
}
```

Both futures and both `Ready` arms borrow `user`. Nothing is cloned, and no `'static` bound or `Arc` appears; the implementation section explains why that works.

The two queries are also in flight at the same time, just as sibling components already render concurrently today. And the two constructs do not coordinate: each one re-renders when its own future completes, without waiting for the other. If recommendations resolve before orders, the recommendations list swaps in while the orders skeleton is still showing.

`live if let` fits when the pending state should render nothing:

```rust
live if let Deferred::Ready(count) = defer(unread_count(cx)) {
    <span class="badge">(count?)</span>
}
```

The badge appears when the count arrives.

### Ownership

A view consumes the values it renders: interpolating an owned value moves it into the output, and props move. That stays true everywhere except one place, the inside of a `live` construct.

```rust
let title = page_title(&user);
let heading = orders_heading(&user);

view! {
    // Outside a live construct, a view consumes values as today:
    // `title` moves into the output.
    <h1>(title)</h1>

    live match defer(orders(cx, &user)) {
        // Borrowing an outside value costs nothing.
        Deferred::Pending => {
            <h2>(&heading)</h2>
            <div class="h-32 animate-pulse rounded-lg bg-muted"></div>
        }
        // Moving one in does not compile: `(heading)` would consume a
        // value the arm does not own. A prop that takes ownership gets
        // an explicit copy.
        Deferred::Ready(orders) => {
            order_panel(heading: heading.clone(), orders: orders?)
        }
    }
}
```

Arms run once per state, so an owned value from outside the construct cannot move into an arm's output; the compiler rejects it. Borrow it instead, which is free, or `.clone()` it where the arm needs its own copy, like the prop above.

The rejection is the point. Each run of an arm that keeps an outside value in its output needs a copy of it, and the clone makes every copy visible in the source. Duplication only happens where the code says `.clone()`. This is the rule for Rust's `Fn` closures, and not by analogy: the arms compile to a closure that runs once per state, so a body that may run again can borrow its surroundings but not consume them.

Everywhere else move semantics hold: content outside `live` constructs, values created inside an arm, and the deferred output itself, which arrives owned.

### The Reactive View Handle

A `Defer` is not the only reactive expression. The second is the value views themselves evaluate to: a `view!` expression and a component call both evaluate to a reactive view handle, the same type. A handle is not rendered output; it stands for one render of its content, and it is an ordinary value: bind it, pass it as a prop, splice it into a view.

```rust
let feed = view! { activity_feed() };

view! {
    <aside>
        (feed)
    </aside>
}
```

A `view!` expression is an anonymous component: the handle it evaluates to is exactly what a call like `activity_feed()` evaluates to, minus the name and the props. Three consequences follow from the handle standing for a render instead of containing one.

There is no `?` on the binding, unlike today. Nothing has failed when a handle is created. A failure happens inside the render the handle stands for and travels through the handle, bubbling out where the handle is spliced, just as a plain component invocation bubbles its errors today. Catching it somewhere other than the splice point is the errors section's topic.

A handle follows the same start rule as a `Defer`: it is lazy, and the render runs once a view consumes it. Splicing `(feed)` is that consumption.

And a handle is cheap to clone. A clone is another reference to the same render, never a second run, so a handle can be spliced in more than one place while its content renders exactly once.

### What Re-Runs

A reactive expression delivering a new state is a reactive event. When one fires, the arms of the consuming construct re-run with the new state, and nothing else runs. Every other expression, component body, and sibling region ran exactly once; the swap splices the new arm output into their existing output by reference. The framework never re-runs code behind your back. Whether anything else on the page defers cannot change how often a component's code runs.

An arm, in turn, is a full view scope: it can declare locals, loop, and invoke components, and those components can defer data of their own. It is also an async scope, so it can load data of its own with a plain `.await`; the await delays only that arm's output. That last point makes one practice worth avoiding: the pending arm renders before the page is sent to the browser, so an `.await` there delays the first response it exists to unblock. Await in the ready arm; the skeleton should render without waiting on anything.

Let's work through some specific examples. First, arms nest. A component invoked from a ready arm can defer data of its own, and the page loads in layers:

```rust
#[component]
async fn product_page(cx: &Cx) -> Result {
    view! {
        live match defer(product(cx)) {
            Deferred::Pending => product_skeleton(),
            Deferred::Ready(product) => {
                let product = product?;
                <h1>(&product.name)</h1>
                <p>(&product.description)</p>
                reviews(product_id: product.id)
            }
        }
    }
}

#[component]
async fn reviews(cx: &Cx, product_id: ProductId) -> Result {
    view! {
        live match defer(reviews_for(cx, product_id)) {
            Deferred::Pending => review_skeleton(),
            Deferred::Ready(reviews) => {
                for review in reviews? {
                    review_card(key: &review.id, review: review)
                }
            }
        }
    }
}
```

The reviews query needs the product id, so it cannot start before the product has loaded. The data flow is the entire orchestration. The response arrives in layers:

```
first paint    the page shell, product skeleton
first swap     product details, review skeletons
second swap    the reviews
```

Each layer paints as soon as it can. Anything unrelated elsewhere on the page streams on its own schedule; regions never wait on one another.

The next example is the flip side of "only the arms re-run": everything inside the arms is inside the re-run scope, whether or not it depends on the deferred data:

```rust
#[component]
async fn dashboard(cx: &Cx) -> Result {
    view! {
        live match defer(drinks(cx)) {
            Deferred::Pending => {
                <div>
                    activity_feed()
                    <div>"loading"</div>
                </div>
            }
            Deferred::Ready(drinks) => {
                <div>
                    activity_feed()
                    <div>"done"</div>
                </div>
            }
        }
    }
}
```

`activity_feed` loads from the database and appears in both arms, so the reactive event creates a fresh instance and runs its body again, database call included. The re-run scope is exactly the arms, so the fix is visible in the source too: shrink the arms to the region that depends on the data.

```rust
view! {
    <div>
        activity_feed()
        live match defer(drinks(cx)) {
            Deferred::Pending => { <div>"loading"</div> }
            Deferred::Ready(drinks) => { <div>"done"</div> }
        }
    </div>
}
```

Now the database call runs exactly once for the whole response, no matter when the `defer` fires: the first render started `activity_feed`, its output is in the buffer, and the swap changes only the match's slot next to it. The answer to "how often does this run" is once, and the code says so.

When the arms need to wrap shared content differently, build the handle once and splice a clone into both:

```rust
let feed = view! { activity_feed() };

view! {
    live match defer(drinks(cx)) {
        Deferred::Pending => {
            <div class="opacity-50">(feed.clone()) <div>"loading"</div></div>
        }
        Deferred::Ready(drinks) => {
            <div>(feed.clone()) <div>"done"</div></div>
        }
    }
}
```

The clones share one render, so `activity_feed` still ran once no matter which arm shows it; the first splice, in the pending arm, is what started it. This holds even if `activity_feed` has `defer`s of its own. Work is cancelled only when nothing references its output anymore, and `feed` stays referenced, by the local and by whichever arm is showing, so an outer swap neither cancels nor restarts it, and its own swaps keep landing in the arm on the page.

When the duplicate work hides inside functions called from both arms, `#[memoize]` removes it, in its intended role: an opt-in cache, not a page-wide requirement. It also covers duplication inside a single evaluation, such as racing two calls of the same query: memoized functions share one in-flight future among concurrent callers.

A future option is to make this automatic at the component level. The component identity system can already tell that the `activity_feed()` in the new arm is the same invocation as the one in the replaced arm. When the props also compare equal, the framework could carry the existing instance across the swap, output and pending work included, instead of running the body again. Memoization only skips runs, so it cannot break the rules above; it is a caching layer the design leaves room for, not something the first cut needs.

### Errors

The `?` in the examples above is the whole error story on the way up: a failed load bubbles out of the component, through the page, into the layouts, as today. This holds whether the failure happens before the response starts or halfway through the stream: where an error lands does not depend on when it happens.

What is new is where errors can be caught, and it needs no new construct: a reactive view handle is a reactive expression like `defer`, so a `live match` can consume one. Different reactive expressions carry different states, and a handle's states are the component's declared return type, `Result<View>`: the `Ok`, carrying the rendered view, arrives when the component hands its view over, the same wait as a plain invocation today, so there is no pending arm to write; a failure that climbs out of the component later arrives as an `Err`. The declared signature is the reactive contract in miniature: a `#[component]` written `-> Result<View>` states exactly what a `live match` on its invocation will see. Written plainly, `drink_grid()` bubbles its errors implicitly, and consumed with `live match`, its `Result` is handled in place:

```rust
live match drink_grid() {
    Ok(grid) => { (grid) }
    Err(_) => {
        <p class="text-muted">"The menu is unavailable right now."</p>
    }
}
```

Before the first paint this is an ordinary catch. After it, a failing load inside the grid climbs to this construct, the `Err` arm runs, and the swap replaces the grid; every load still pending inside the replaced region is cancelled.

Layouts catch the same way. A layout's slot is the page's reactive view handle, so its type changes spelling from `Result` to `Slot`; what happens to layouts written against today's `slot: Result` signature is listed with the open questions:

```rust
#[layout("/")]
async fn root_layout(slot: Slot) -> Result {
    view! {
        <html>
            <body>
                live match slot {
                    Ok(content) => { (content) }
                    Err(error) if error.downcast_ref::<NotFoundError>().is_some() => {
                        (StatusCode::NOT_FOUND)
                        <h1>"Nothing here"</h1>
                    }
                    // The last arm rethrows by evaluating to the error,
                    // which moves the catch to the next layout out; its
                    // spelling is an open question.
                    Err(error) => { ... }
                }
            </body>
        </html>
    }
}
```

If nothing catches, the framework's error view swaps in as a whole-page update.

One streaming constraint reaches into error handling: the status code is locked in by the first chunk. A catch that fires mid-stream still swaps in its error UI, but a `(StatusCode::NOT_FOUND)` it renders can no longer change the response's already-sent status. The streaming behavior section returns to this.

Inside an arm, plain `match` still gives local error UI without any of this:

```rust
Deferred::Ready(drinks) => {
    match drinks {
        Ok(drinks) => {
            for drink in drinks {
                drink_card(key: &drink.slug, drink: drink)
            }
        }
        Err(_) => {
            <p class="text-muted">"The menu is unavailable right now."</p>
        }
    }
}
```

No error boundary component, no second mechanism: `Result`, `match`, and `?`.

### One Skeleton for Many Loads

Leaf components should load their own data, but the loading UI often belongs higher up: one skeleton covering a section, not a spinner per leaf. Component calls being reactive values make this a composition. A `settled` adapter (working syntax, like `live`) wraps one handle or a tuple of them and reports `Pending` until nothing beneath is still loading:

```rust
live match settled((drink_grid(), activity_feed())) {
    Deferred::Pending => { section_skeleton() }
    Deferred::Ready((grid, feed)) => {
        (grid?)
        (feed?)
    }
}
```

Both components start during the first render, when the construct consumes them, and load concurrently; there is no waterfall. Their own pending arms never ship: the section renders as one skeleton and swaps in once, complete. The first chunk does not even wait for these components to render, since the skeleton is ready immediately.

A leaf that leaves its loading UI entirely to its parents renders nothing while pending:

```rust
#[component]
async fn drink_grid(cx: &Cx) -> Result {
    view! {
        live if let Deferred::Ready(drinks) = defer(drinks(cx)) {
            for drink in drinks? {
                drink_card(key: &drink.slug, drink: drink)
            }
        }
    }
}
```

The same shape scales to the whole page: a layout that consumes `settled(slot)` renders one loading page until the content beneath it settles. Pending stays in `Deferred`, errors stay in `Result`, and an ancestor opts in by wrapping.

### Streaming Behavior

When a render finishes with no `Pending` in it, the response is built and sent whole, exactly as today; a page without `defer` pays nothing. Otherwise the first chunk goes out as soon as the page has rendered around its skeletons, carrying the status code and headers that render declared, and the connection stays open. Each fired `defer` becomes a swap; reactive events that happen close together coalesce into one chunk. When nothing deferred remains, the stream closes.

Two constraints are inherent to streaming. The status code and headers are locked in by the first chunk, so declarations from later content are discarded. And a redirect that surfaces mid-stream cannot become a `Location` header, so it is delivered as a swap instruction that makes the client navigate.

Contexts that cannot stream need no second implementation: rendering to completion instead of responding at the first paint produces the final document in one piece. That fits crawlers and JS-less clients, tests, and renders that are not HTTP responses at all, such as mail bodies. The result is byte-for-byte the document a streaming client ends up with.

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

Diffing the new document against the previous one is then a hash comparison per boundary. Only boundaries whose hash changed are written to the stream; unchanged regions, usually most of the page, are never retransmitted and their DOM is never touched. Structural changes fall out of the same rule: a boundary that appears, disappears, or moves changes its parent's placeholder sequence and thus the parent's hash, so the parent swap carries the new structure. The live render knows which slots refilled, so hashing can skip boundaries with no changed slot; like keeping unchanged descendants out of a parent swap, this is an optimization, not a requirement.

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

## Alternatives

Two other designs were worked through before this one. Both deliver streaming; both were rejected for what they do to error handling or to local reasoning.

### A Direct Suspense Port

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

The root cause is that Suspense detaches a continuation from the call stack that created it. This proposal refuses to do that: the continuation and its stack stay together as one live value, which is what a Rust future is.

### Re-Rendering From the Root

There is a simpler way to keep a call stack available: re-create it. An earlier revision of this proposal did that. When any deferred future completed, the framework re-invoked the page function, layouts and all, and each `defer` whose future had completed returned `Ready` on the new pass. Errors work well under that model: the pass that sees an error is a full render with a full call stack.

The problem is that it breaks local reasoning. How often a component body runs per request is no longer decided by the component; it is decided by whether anything else on the page defers. A component written to run once starts running several times because a teammate adds a `defer` to a sibling, or because a third-party component defers internally. Nothing in the affected component's code, its signature, or the review diff shows the change.

The cost follows. If everything may run many times, every data-loading call must be memoized, so `#[memoize]` turns from an optimization into a requirement. Forgetting it has no visible symptom: the page works, it just silently repeats database queries and API calls.

Rust has a close precedent: async cancellation. Any `.await` is a point where the enclosing future may be silently dropped, the caller decides, and years of subtle cancellation bugs show what happens when code cannot tell locally how it will be executed. Re-rendering from the root builds the same kind of hazard into rendering.

The contrast surfaces throughout the proposal above. Ownership: every pass of a root re-render rebuilt every value on the page invisibly, where a `live` arm makes each copy it keeps a visible `.clone()`. Execution count: the answer to "how often does this run" was once per pass unless everything was memoized, where the reactive render's answer is once, decided by the code. Group loading UI: the only way to express a section-wide skeleton was to make "still loading" an error, caught in layouts behind an `is_deferred()` check, which put pending and failure in one channel; `settled` keeps pending in `Deferred` and errors in `Result`. And bookkeeping: a `Defer` that owns its future needs no call-site identity and no `key:`, where root re-rendering had to re-associate every `defer` with its previous pass.

## Implementation

The proposal above is behavior; this section is machinery. It answers three questions: who re-runs a fired `match`, how its arms can still borrow the component's locals, and how the new output reaches the wire. The expansions are simplified and the names are provisional.

### The Live Render

The tempting place to store a reactive `match` is inside the `View`: keep the arms as a closure, call it again when the future completes. That cannot work. A `View` outlives the function that built it, so anything stored inside one must be `'static`, and the closure could not borrow `user` or anything else from the component body. That is the Suspense trap again.

So the ownership is inverted. The `View` stays what it is today: a cheap, cloneable, inert handle into the instruction buffer. The re-run code stays where it is written, in the component's body, inside the future executing it. What changes is that the future does not return when its view is built: it hands the view to its parent and keeps running until nothing inside it can change anymore. Locals like `user` live on the future's frame, and arms that reference them after an await are exactly the self-referential shape `async fn` exists to compile.

The whole page render is one live future: layouts wrap the page, components nest in components, each level drives the levels below it. A completed deferred future wakes the task; the poll descends to the construct that owns it, called a reactive node; the node re-runs its arms and points its slot in the instruction buffer at the new output. Every enclosing component embedded that slot by reference, so their rendered output changes without any of their code running.

This answers the questions a stored-closure design cannot. Where is the continuation stored? In its caller: a component's future is boxed into the parent's `RefreshSet`, a local of the parent's own future, and so on up to the root future, which the router's response task owns. What is its lifetime? Its position in that tree. Props move into the future like arguments into any async call: an owned `String` by value, a `&str` borrowing from the parent's live frame. Nothing needs `'static`. Who runs it? The one task polling the render; there is no spawning and no detached work.

### Inside `view!`

Today `view!` expands to three phases: a hoist that evaluates every expression in source order and binds component render futures, a `try_join!` that awaits the components together, and a synchronous burst that lays down the view's instruction block. The new expansion keeps all three, routes them through a `RefreshSet` (defined below), and registers refreshes. For the `profile` component from the guide, reduced to the `<h1>`, an `avatar(user: &user)` child, and the orders `defer`:

```rust
// Simplified: what the `view!` in `profile` expands to.
{
    // Hoist: evaluate expressions in source order, as today.
    let __expr0 = &user.name;

    // A component invocation: reserve a slot registered with the set's
    // barrier and start the child. The child fills the handover when its
    // render phase finishes, then stays live in `__refresh` while it has
    // pending work of its own.
    let (__child0, __handover0) = __refresh.reserve_child();
    let __props0 = avatar::props_builder().user(&user).build();
    __refresh.push(avatar::render(__cx, __props0, __handover0));

    // A reactive node: a reserved slot, the reactive expression, and the
    // arms as a closure that can run for any of its states. The set
    // argument is where components started by an arm adopt.
    let (__node0, __node0_slot) = internal::reserve();
    let mut __r0 = defer(__cx, orders(cx, &user));
    let __node0_arms = async |__state: Deferred<_>, __set: &mut RefreshSet<'_>| {
        Ok(match __state {
            Deferred::Pending => internal::block(__cx, |__b| {
                __b.markup(&"<p class=\"skeleton\">Loading orders...</p>");
            }),
            Deferred::Ready(orders) => {
                // An arm is a nested view scope with its own hoist, join,
                // and burst.
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

    // First evaluation with the expression's current state, adopting into
    // the body's own set so skeleton children join the first paint. A
    // `defer` is lazy until consumed: `current` polls its future for the
    // first time here, so data already at hand reports `Ready` and the
    // node retires immediately.
    let __first = __node0_arms(__r0.current(), &mut __refresh).await?;
    __node0_slot.fill(__first);

    // The node's refresh: each later state re-runs the arms into a set of
    // their own and swaps the slot. Pushed, not run: `__refresh` polls it
    // from now on, so the deferred future makes progress while the rest
    // of the page renders. A `Defer` changes at most once, so the loop
    // body runs at most once.
    __refresh.push(async move {
        while let Some(__state) = __r0.changed().await {
            let mut __arm = RefreshSet::new();
            let __view = __node0_arms(__state, &mut __arm).await?;
            __arm.barrier().await?;        // the arm's children hand over
            __node0_slot.refill(__view);   // the swap: marks the buffer dirty
            __arm.run().await?;            // nested defers keep streaming
        }
        Ok(())
    });

    // Join: wait for every handover reserved above. This replaces today's
    // `try_join!`; refresh work stays live in the set.
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

Three things changed. The child component is not awaited to completion: it fills a reserved slot when its render phase ends, and `try_join!` became a barrier that waits only for those handovers. The `match` became a reactive node whose slot, expression, and arms are plain local values, all free to borrow `user`. And the refresh, the only code that will ever run again, is registered instead of executed.

The nested `RefreshSet` in the pushed refresh makes arms recursive: components started by the `Ready` arm, and any `defer` they contain, live inside the node's own refresh future. A chain like the product page in the guide is this structure nested twice. The `__arm.barrier()` before the `refill` guarantees a swap ships complete content: the arm's children have handed over, even if they contain fresh skeletons of their own.

### The Reactive Contract

The expansion consumes `__r0` through two methods. That pair is the entire interface between a reactive expression and the view that consumes it:

```rust
/// A value a view can consume reactively: a state to render now, and zero
/// or more replacement states later.
pub trait Reactive {
    /// The state the consuming construct's arms are written against.
    type State;

    /// The state to render now. Called once, by the consuming node's
    /// first evaluation.
    fn current(&mut self) -> Self::State;

    /// The next state, or `None` once no further change is possible.
    /// Retiring is what lets the node, and eventually the page, complete.
    async fn changed(&mut self) -> Option<Self::State>;
}
```

`Defer` is the first implementation: it owns its future, `current` gives the future its first poll and reports `Pending` or `Ready`, and `changed` awaits the future, yields `Ready` once, and retires. Laziness falls out of the ownership: nothing polls the future before a consuming node calls `current`. The methods are for generated code, not applications; that is what keeps a `Defer` opaque outside a view. Nothing in the node's compilation is specific to `defer`: a reactive expression is a stream of states, and a reactive node renders each state into the same slot.

States are delivered by value. The `Ready` arm receives the future's output owned, just as `.await` would deliver it, with no `Clone` bound and no cached copy; that works because each state is consumed exactly once, by one run of the arms. Rendering one `Defer` in two places would need sharing, which is the open question about consuming by reference.

Component calls are the second implementation. A call like `drink_grid()` evaluates to a handle that owns the marker and props and implements `Reactive` with `State = Result<View>`: the first state arrives when the component hands over its view, the same wait as the barrier, and an error climbing out of the component later arrives as an `Err` state. A plain invocation in a view is sugar for consuming the handle in bubble mode, render the `Ok`, pass the `Err` up; `live match` on the call is the catch. Outside a view, the props builder produces the same handle.

The handle clarifies the split between the proposal's two view types. `View` is the rendered view: the inert, ref-counted block handle it is today, appearing as the payload inside a state. The reactive view handle is the value in circulation, and it is one concrete type, provisionally `ViewHandle<'a>`: component calls, `view!` expressions, and the layout slot all evaluate to it. No per-component handle type is needed, because a call's props move into the handle at creation and the future is boxed there; the box is the same one allocation per child the `RefreshSet` already pays, moved from consumption to creation. `'a` is the borrow of the caller's frame. `State = Result<View>` is the component's declared return type verbatim, so the signature the user writes documents the states while the macro rewrites the function to the handover form below; a `view!` expression produces the anonymous case, the same type with no component marker.

One start rule follows: a handle is lazy exactly like a `Defer`, and its render runs once a view consumes it. This also removes any special case for a component body's trailing `view!`: the body returns a handle like any other expression, and the generated epilogue consumes it, awaits its first state, and fills the handover. The expansions in this section show the fused form of that rule: a plain invocation and a tail-position `view!` are consumed by the surrounding view the moment they are created, so the macro inlines creation and consumption; a handle bound to a local is the unfused case.

Handles are cheap to clone; a clone is another reference to the same render through the same counts that keep it alive, so cloning never re-runs a body, and embedding clones in several places is plain bubble-mode consumption. Only `live` consumption is exclusive: states are delivered by value to one construct, which is the sharing question below.

Adapters compose on top, because handles are just `Reactive` values. `settled` lifts a handle, or a tuple of them, into a `Deferred`-shaped state that stays `Pending` until nothing beneath the handles is still loading, which the liveness machinery already tracks; that is the group-skeleton pattern in the guide. One wrinkle for prototyping: a handle's first state is not available synchronously, so `current` becomes an await, or the contract grows a first-state step. `Defer` is unaffected either way.

The trait is also the extension point. A signal read is a `Reactive` whose `changed` keeps yielding as the client changes the value, and retires when the connection closes. Expressions that fire more than once need one refinement the `defer` node skips: a new state should cancel the replaced arm's pending work instead of waiting for it, so `changed` must be raced against the nested set rather than run after it. That refinement arrives with signals, not with `defer`.

### Inside `#[component]`

`__refresh` is declared by the `#[component]` expansion, which is where the future learns to outlive the view it produces:

```rust
// Simplified: the future `#[component]` generates for `profile`.
fn render<'cx>(
    cx: &'cx Cx,
    props: ProfileProps<'cx>,
    __handover: Handover,
) -> impl Future<Output = Result<()>> + Send + 'cx {
    async move {
        // Collects the body's live work: children and node refreshes.
        // `view!` expansions in the body push into it.
        let mut __refresh = RefreshSet::new();

        // The body, unchanged. A `?` here fails before the yield; the
        // parent's barrier sees the error instead of a handover.
        let user = current_user(cx).await?;
        let __view = { /* the `view!` expansion above */ };

        // The yield: hand the finished view to the parent, then keep
        // going. This is the line where today's generated code returns.
        __handover.fill(__view);

        // The refresh phase: drive children and reactive nodes until
        // none have work left. With nothing pushed, this completes
        // immediately and the whole future was today's behavior.
        // `user` is alive across this await; that is the point of not
        // returning.
        __refresh.run().await
    }
}
```

The view travels through the handover, not the return value; the future's output is the component's final status, which is how error transitions bubble. The future completes when nothing inside it can change anymore, so a page without `defer` completes on its first pass and streaming costs nothing.

### The `RefreshSet`

The set is small: a `FuturesUnordered` of boxed, deliberately non-`'static` futures, plus handover accounting for the barrier. It lives in `topcoat_view::internal` next to `reserve()` and `try_join!`:

```rust
/// Collects a component body's live work: the render futures of child
/// components and the refresh futures of its reactive nodes.
///
/// `'body` is the component body's lifetime. Everything in here may borrow
/// the body's locals, which is legal because the set is itself one of them
/// and never escapes.
pub struct RefreshSet<'body> {
    /// One boxed future per child and per node, polled FuturesUnordered
    /// style: only entries whose waker fired get re-polled, so one
    /// completion does not re-poll fifty pending siblings.
    entries: FuturesUnordered<Pin<Box<dyn Future<Output = Result<()>> + Send + 'body>>>,
    /// One flag per child handover. The barrier is down when every flag
    /// is set.
    handovers: Vec<Arc<AtomicBool>>,
}

/// What a child fills instead of returning its view: the parent's reserved
/// slot, plus the flag the parent's barrier watches.
pub struct Handover {
    slot: ViewSlot,
    filled: Arc<AtomicBool>,
}

impl Handover {
    pub fn fill(self, view: View) {
        self.slot.fill(view);
        self.filled.store(true, Release);
        // No waker needed: this runs inside a poll of the same task that
        // polls the barrier below.
    }
}

impl<'body> RefreshSet<'body> {
    /// Reserves a child's slot and registers it with the barrier.
    pub fn reserve_child(&mut self) -> (View, Handover) {
        let (placeholder, slot) = reserve();
        let filled = Arc::new(AtomicBool::new(false));
        self.handovers.push(filled.clone());
        (placeholder, Handover { slot, filled })
    }

    /// Registers work: a child's render future or a node's refresh. The
    /// two are the same to the set; a child differs only in having a
    /// handover flag.
    pub fn push(&mut self, work: impl Future<Output = Result<()>> + Send + 'body) {
        self.entries.push(Box::pin(work));
    }

    /// Drives the set until every reserved handover is filled. Node
    /// refreshes are polled too, they just are not waited on; this is
    /// what starts deferred futures during the render phase.
    pub async fn barrier(&mut self) -> Result<()> {
        poll_fn(|task| {
            while let Poll::Ready(Some(done)) = self.entries.poll_next_unpin(task) {
                done?; // a child failed before handing over
            }
            if self.handovers.iter().all(|filled| filled.load(Acquire)) {
                return Poll::Ready(Ok(()));
            }
            Poll::Pending
        })
        .await
    }

    /// Drives the set to completion. Empty set: completes immediately,
    /// which is the non-streaming fast path.
    pub async fn run(mut self) -> Result<()> {
        while let Some(done) = self.entries.next().await {
            done?; // first error wins; dropping `self` cancels the rest
        }
        Ok(())
    }
}
```

Three properties matter:

- The barrier needs no synchronization. The whole render is one task, and the children being waited on are entries of this same set, so a `Handover::fill` can only happen inside the `poll_next` calls above. Checking the flags after each poll sweep is exact; the `AtomicBool` exists to satisfy `Send`, not because threads race.
- `run(self)` must consume the set. It is declared at the top of the generated body but holds borrows of locals declared after it; the borrow checker only accepts that if the set is consumed before those locals drop, and the epilogue's `run()` is that consumption. Consuming also makes cancellation one drop: `FuturesUnordered` takes every child and node future with it, recursively.
- Boxing is one allocation per child and per node, comparable to what props and blocks already cost. The `Vec<Arc<AtomicBool>>` could collapse into one shared counter; the flags are just easier to follow.

### Liveness and Cancellation

Work should stop when its output can no longer reach the page, and reference counting decides when that is. `View` handles and the blocks they point at are ref-counted: splicing a view into a block is a reference, and so is holding one in a variable. A producer's future stays in its `RefreshSet` while its output has at least one reference; at zero the set drops the entry on its next sweep, and dropping a future is cancellation in Rust.

Counts fall at `refill`. Replacing a slot's block releases that block's references, recursively through everything it spliced. A skeleton's children lose their last reference the moment the ready arm replaces them, so their loads stop. The shared `feed` from the guide keeps references, one from the local and one from whichever arm is showing, so it keeps running. Reachable means alive, with no marking pass and no special cases.

Two consequences follow. A handle held in a variable keeps its render alive even while no arm shows it, until the variable drops at the end of the component's future; that is ordinary Rust drop timing, and the shared `feed` relies on it. And a block at count zero is known garbage, which gives buffer compaction a place to start.

### The Driver

The router composes the page and its layouts into one live render, the same call chain it builds today, and drives it:

```rust
// Simplified: the router driving a streaming response.
let mut render = pin!(compose(layouts, page, cx));

// First paint: the root hands over the document when its render phase
// finishes. Deferred futures make progress from the moment the render
// consumes them, so slow queries overlap the first paint instead of
// starting after it.
let first = render.first_view().await?;
send_chunk(first.html, first.status_code, first.headers);

// The render stays alive until nothing can change anymore. Each pulse
// means slots were refilled; re-executing the instruction buffer is
// framework code, no user code.
while let Some(_changed) = render.next_change().await? {
    let html = render.execute_buffer();
    send_chunk(diff_boundaries(&mut baseline, &html));
}

// The render future completed: everything resolved and shipped.
```

Change signaling is not the `RefreshSet`'s job. `ViewSlot::refill` marks the shared instruction buffer dirty, and `next_change` polls the live render, reports when the dirty bit is set, and ends when the future completes. Reactive events that land in the same poll pass coalesce into one chunk for free. The instruction buffer stays with the live render for the whole response, since refills keep writing to it.

### Error Transitions

On the first render, a failure is the `?` before the yield in the `#[component]` expansion: the future returns `Err` instead of filling its handover, the parent's barrier propagates it, and the error climbs the live call chain as it does today.

After the first paint, a failure is the `?` inside a pushed refresh: `orders?` failing when the `Ready` arm runs. The node's refresh future produces the error, so the component's `run()` produces it, so the component's future produces it, and so on up the join tree that mirrors the call chain. A component that invoked the failing child as a plain call passes the error along without any of its code re-running, the same as the implicit `?` on a first render.

Catching is a reactive event. A `live match` on a component call owns that component's handle; an error climbing out of the component arrives as the handle's next state, the node re-runs its arms with `Err`, and the swap replaces the region, dropping the replaced subtree's remaining work. A layout's slot is such a handle, so layouts catch with the same construct and no special machinery. No component body anywhere re-runs to deliver an error, and if nothing catches, the framework's error view swaps in as a whole-page update.

## Requirements on Application Code

The design leans on one rule that Topcoat already imposes: page renders are side-effect free. Concurrent rendering already forbids components from depending on execution order or communicating through shared state, and prefetching already means a page may render without a user looking at the result. This proposal adds only time to the same contract: a `live` arm may run well after the surrounding body finished, and a pending arm's output is discarded when the ready arm replaces it. Code that treats rendering as a pure function of its inputs does not notice. The contract is lighter than under the re-render design, which re-executed the entire page.

Boundary diffing adds a softer expectation: renders should be deterministic, because a boundary that renders differently from the same data hashes differently and gets re-sent and re-swapped for nothing. Freshly generated random ids, timestamps rendered mid-page, or iteration over unordered maps cause spurious swaps. The result is correct but wasteful, and a swap replaces DOM, which discards focus, scroll position, and input state inside the region. For now this is a documentation concern: the user should keep boundary content stable. Tooling, such as a dev-mode warning when a boundary's hash changes although no slot inside it refilled, can come later.

## Open Questions

**Reactive syntax.** `live` is working syntax, not final. As a contextual modifier keyword it follows Rust's own pattern (`async`, `unsafe`, `const`, and `gen` blocks), with C#'s `await foreach` and JavaScript's `for await` as cross-language precedent; runner-up spellings are a `#[live]` attribute and a Maud-style sigil such as `@match`. Also to settle: which constructs can be `live` (`match` and `if let` first); whether a reactive expression in node position, for a deferred fragment of text, is worth having; the name and final shape of the `Reactive` trait; and whether a `Defer` should also be consumable by reference, so one load can render in more than one place.

**Generated-code plumbing.** The `RefreshSet` sketch settles the shape; the generated code around it needs prototyping. The borrow checker must accept the collector-before-locals pattern in real bodies, the arms closure needs a workable `async` closure signature, and `Component::render` changes from return-once to yield-then-continue. In a `view!` outside a `#[component]`/`#[page]`/`#[layout]` transform, reactive nodes should be a compile error, and component invocations should keep completion semantics: the expression awaits the whole subtree, which is the blocking mode above.

**Reference counting details.** Liveness and cancellation rest on ref-counted views and blocks. To settle: the cost of the release walk at `refill`; whether the pathological cycle, two slots filled with views that reference each other's placeholders, needs a runtime check or just a documented rule; and whether a handle still held in a variable after its content left the page needs tooling, since the variable keeps the producer alive and running until it drops.

**Component handles.** A component call evaluating to a `Reactive` handle needs: the spelling for building one outside a view (probably the props builder); the contract change for a first state that is not synchronously available; the `settled` adapter's subtree observation, fed by the reference counts; the rethrow arm in a catching `live match`, since view matches stay exhaustive; the layout slot's move from `Result` to a handle type, including what happens to layouts written against today's signature; and names for the adapter set, `settled` and any joins, which are working syntax throughout.

**The `view!` expression type.** A `view!` expression evaluating to a handle instead of today's `Result<View>` changes what a binding like `let feed = view! { ... };` means: the `?` disappears and errors travel through the handle. To settle: the handle type's name and exact shape (`ViewHandle<'a>` is provisional); confirming the macro can fuse creation and consumption for plain invocations and tail-position `view!`, as the expansion sketches assume; and whether two `live` constructs may consume clones of the same handle, the sharing question from the reactive contract.

**Batching.** Completions that arrive in one poll pass already coalesce into one chunk. Whether to add a short window that also coalesces near-simultaneous completions across wakes is undecided.

**Limits.** A deadline per request is probably wanted: when it expires, the framework stops polling the render, the stream closes, and pending regions keep their skeletons. Dropping the render future cancels all outstanding work, so enforcement is one drop.

**Buffer growth.** Every refill appends new blocks and orphans the ones they replace. For streaming this is bounded by the number of reactive events, but if the live render later services long-lived updates, compaction or block reuse becomes worth designing.

## Why This Pays Off Later

Streaming is the first consumer of the boundary machinery, not its ceiling. What this proposal actually builds is a general operation: render the page, diff its boundary tree against whatever the client already holds, ship only the difference. Within one request, the baseline is what the response has already carried. But nothing about the diff cares where the baseline comes from, and that one degree of freedom turns the same machinery into the foundation for the two features Topcoat most wants next. Implementing them is out of scope here; designing the diff against an arbitrary baseline instead of hardwiring it to the previous chunk is cheap now and is what keeps these doors open.

### Client-Side Navigation

The client knows its current boundary tree, identities and hashes, because the server rendered it. On a client-side navigation, the client sends that tree to the server, tentatively in an `X-Topcoat-Boundaries` header, and the server diffs its very first render of the target page against the client's state instead of a previous chunk. Everything the two pages share, the document shell, the navigation, the footer, every layout the routes have in common, hashes identically and never travels. The response carries only the boundaries that actually differ.

This composes with `defer` for free. A navigation response can arrive as a stream like any other: the changed regions come down first as skeletons, then fill in as their data resolves. Navigation to a slow page feels instant, because the instant part really is sent instantly.

What makes this remarkable is what it does not require. This is the experience single-page application frameworks exist to provide, and the standard price is enormous: application logic compiled for the browser, a hydration step, a client-side router, and a second rendering model that the server-rendered one must stay consistent with. Here it falls out of a header and a diff the server already knows how to compute. The server remains the only place rendering happens, the browser holds nothing but the swap script, and a page written for a full document load works for client navigation without a single change. Prefetching gets cheaper for the same reason: a prefetched navigation response is small because it excludes everything the client already has.

### Signals Without Shards

Shards exist because sometimes the markup itself needs the server: fresh search results as the user types. Today that means extracting the markup into a `#[shard]`, a separate server endpoint with its own arguments, its own untrusted-input surface, and code pulled out of the page that owns it.

The refetch model absorbs this. The server can track which signals a page reads during a render. When one of them changes in the browser, the client refetches the page itself, sending the current signal values up in a header. The server prefills the signal reads with the client's state and re-renders the page, which is just an ordinary render of ordinary code. The boundary diff then does what it always does: regions whose output did not depend on the changed signal hash identically and stay untouched, and only the regions that genuinely changed travel back.

The live render adds a second route to the same destination. A signal read inside `view!` is just another implementation of the reactive contract: one whose `changed` keeps yielding as the client updates the value. Over a connection that keeps the render alive, a signal change would re-run only the arms that read it, with no refetch at all. Whether refetch or a live connection fits better, and where shards end up, are later discussions; both ride the same boundary diff and the same reactive nodes.

### One Model for Everything

Step back and every kind of update has collapsed into the same shape. A first load, a deferred piece of data arriving, a client-side navigation, a signal change: each one is "render, diff against the client, ship the difference". One wire format, one swap script, one server code path, and one rule for application code, which Topcoat demands already: renders are functions of their inputs, free of side effects. The page becomes a pure function from route and state to HTML, and what makes calling it cheap is not memoizing everything but the live render itself, which re-runs only the code whose inputs changed.

That is why this design is worth its cost. The constructs are small on the surface, the live render is real machinery underneath, and together they put the framework on a trajectory where streaming, navigation, and interactivity stop being separate features with separate mechanisms and become one mechanism observed at different moments.
