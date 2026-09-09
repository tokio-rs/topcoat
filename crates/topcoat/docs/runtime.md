Topcoat's runtime makes server-rendered pages interactive without a wasm bundle, a client build step, or a separate frontend. Reactive state and expressions are written inline in [`view!`], type-checked as ordinary Rust, and compiled to JavaScript that ships with the page.

The runtime is **highly experimental** and fairly limited today: expressions support only a small vocabulary of types and methods, and many patterns have no ergonomic answer yet. It will improve in future releases; expect both additions and breaking changes.

# Setup

Interactive pages need the runtime's browser script. `script()` renders the script tag; include it in your document head:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn document() -> Result<impl View> {
Ok(view! {
    <html>
        <head>
            topcoat::runtime::script()
        </head>
        <body></body>
    </html>
})
# }
```

The runtime also needs the router set up for it. [`runtime()`](RouterBuilderRuntimeExt::runtime) mounts the routes the browser script talks to on its own, and the script is served as a Topcoat [asset](../asset/index.html), so the asset bundle must be loaded as well:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
    runtime::RouterBuilderRuntimeExt,
};

pub fn router() -> Router {
    Router::builder()
        .runtime()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build()
}
```

`.runtime()` covers the runtime's own routes only. The endpoints behind your [procedures](#procedures) and [shards](#shards), covered later in this guide, are annotated items that `.discover()` registers like pages and layouts.

# Runtime expressions

A `$(...)` block is a **runtime expression** and can stand wherever a view node can:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <p>"The answer: " $(1.0 + 2.0)</p>
})
# }
```

The expression is type-checked Rust, but it is compiled twice: the server evaluates it once for the initial HTML, and an equivalent JavaScript translation ships with the page, where it can run again without any help from the server.

Because a runtime expression must behave identically in both languages, only a subset of Rust is supported: a small vocabulary of types and methods. `$(...)` is syntactic sugar for the [`expr!`] macro, which documents that vocabulary, how captured variables behave, and the `raw!` escape hatch to hand-written JavaScript.

So far the browser has no reason to run `1.0 + 2.0` a second time; the answer stays `3`. Expressions become useful when they read state that changes: signals.

# Signals

A **signal** is a piece of state that lives in the browser. Create one with [`signal`], passing the request context and a closure producing the initial value, and read it in a runtime expression with `.get()`:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 0.0);

Ok(view! {
    <p>"Count: " $(count.get())</p>
})
# }
```

The initial value is computed once during the server render and serialized into the page; the browser picks it up as reactive state. **From then on the value is user input.** The browser holds it, and anything the server later reads back from the signal **must not be trusted**. A signal belongs to the page, layout, component, or shard body that creates it. It is an ordinary value that is cheap to clone, so it can be captured by any number of runtime expressions in that body's view and handed down to the components it renders as `&Signal<T>`.

In the browser, a runtime expression re-runs whenever a signal it read changes -- the text above updates the moment `count` does, with no server round-trip. Inside an expression you work with a signal through its methods: `.get()` reads the current value and `.set(...)` replaces it. Nothing changes `count` yet, though; that is what event handlers are for.

# Event handlers

An attribute starting with `@` attaches an event handler: `@click`, `@input`, or any other DOM event name. Its value is a runtime expression evaluating to a closure, which runs in the browser each time the event fires. Handlers are where signals change:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 0.0);

Ok(view! {
    <button @click=$(|_e| count.set(count.get() + 1.0))>"+1"</button>
    <p>"Count: " $(count.get())</p>
})
# }
```

Clicking the button runs the closure, the closure updates the signal, and the `$(count.get())` text re-renders. The entire loop happens in the browser.

The closure receives an [`Event`] mirroring the DOM event: fields like `e.target.value`, `e.key`, and `e.client_x`, and methods like `e.prevent_default()`. A typical input handler forwards the element's value into a signal:

```rust
# use topcoat::{Result, context::Cx, runtime::{Event, signal}, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let query = signal(cx, String::new);

Ok(view! {
    <input @input=$(|e: Event| query.set(e.target.value))>
})
# }
```

For the rare event logic the expression vocabulary cannot say, the value can also be a string literal of raw JavaScript: `@click="alert('hi')"`.

`set` is the general write, and a few updates that depend on the current value have a shorter spelling: `toggle` on a `bool` signal, `increment` and `decrement` on an `f64` signal, and `push_str` on a `String` signal. The handler above can therefore be written as `$(|_e| count.increment())`.

# Bind attributes

An attribute starting with `:` is a **bind attribute**: its value is a runtime expression, and the attribute is kept in sync with it. The server renders the initial value like a normal attribute; the browser re-applies it whenever a signal the expression reads changes:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let open = signal(cx, || false);

Ok(view! {
    <button @click=$(|_e| open.set(!open.get()))>"What is Topcoat?"</button>
    <p :hidden=$(!open.get())>"A fullstack Rust framework."</p>
})
# }
```

Combining a bind attribute with an event handler syncs an element and a signal in both directions: `:value` keeps the input showing the signal, and `@input` writes every keystroke back into it:

```rust
# use topcoat::{Result, context::Cx, runtime::{Event, signal}, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let name = signal(cx, String::new);

Ok(view! {
    <input
        :value=$(name.get())
        @input=$(|e: Event| name.set(e.target.value))
    >

    <p>"Hello, " $(name.get()) "!"</p>
})
# }
```

# Procedures

Runtime expressions run in the browser, so they cannot query the database or use Rust beyond the shared vocabulary. When an event handler needs the server, it calls a **procedure**: an async server function invoked from a runtime expression like any other async function:

```rust
# use topcoat::{Result, context::Cx, runtime::{procedure, signal}, view::*};
#[procedure]
async fn double(value: f64) -> Result<f64> {
    Ok(value * 2.0)
}

# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 1.0);

Ok(view! {
    <button @click=$(async |_e| {
        let doubled = double(count.get()).await;
        count.set(doubled);
    })>
        "double it"
    </button>
})
# }
```

The call is an HTTP request under the hood: the arguments travel to the server, the function runs there, and the `.await` resolves to its result. That also means every procedure is exposed as an API endpoint from your server; anyone can call it with any arguments, so inputs can be spoofed and **must not be trusted**. See [`#[procedure]`][procedure] for the details: argument and return types, the `cx` parameter, error handling, and registration.

# Shards

When it is the markup itself that needs the server -- fresh search results as the user types -- use a **shard**: a component that re-renders on the server whenever one of its arguments changes. Arguments are runtime expressions; the browser sends their current values to the server and swaps the returned HTML in place:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{shard, signal, Event}};
# async fn search_products(_cx: &Cx, _query: &str) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl View> {
    let products = search_products(cx, &query).await?;
    Ok(view! {
        for product in products {
            <div>(product)</div>
        }
    })
}

# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let query = signal(cx, String::new);

Ok(view! {
    <input @input=$(|e: Event| query.set(e.target.value))>

    search_results(query: $(query.get()))
})
# }
```

A shard body is ordinary server code, like any component. The re-renders are served by an API endpoint exposed from your server, so a shard's arguments can be spoofed just like a procedure's and **must not be trusted**. See [`#[shard]`][shard] for the details: how re-renders behave, shard state, and registration.

# Reading signals on the server

A signal can also be read in plain Rust, outside any runtime expression, in the body that created it. `.get()` clones the current value and `.read()` borrows it. Both are **tracked reads**: they make the page depend on the signal, so when the signal changes in the browser, the page runs again on the server with the signal's current value and its content is replaced with the result:

```rust
# use topcoat::{Result, context::Cx, router::page, runtime::{Event, signal}, view::*};
# async fn search_products(_cx: &Cx, _query: &str) -> Result<Vec<String>> { Ok(vec![]) }
#[page("/search")]
async fn search(cx: &Cx) -> Result<impl View> {
    let query = signal(cx, String::new);
    let products = search_products(cx, &query.get()).await?;

    Ok(view! {
        <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

        for product in products {
            <div>(product)</div>
        }
    })
}
```

The input keeps working as a client-only binding, and the product list follows it through the server. On the re-run, [`signal`] starts from the value the browser sent instead of computing a fresh one, so the page picks up where the client left off. Reads inside a `$(...)` expression never make the page depend on a signal; they are the client-side path and stay in the browser.

**Every value read on the server is user input and must not be trusted.** The client holds the signal and can send anything that fits its type, so validate the value before acting on it, exactly like a shard argument.

The result is morphed into the page rather than swapped in. Elements that still exist are updated in place, so focus, scroll position, and what the user is typing survive a re-run, and every signal keeps its value. Give the items of a list that can reorder an `id`, so the morph follows each item to its new position instead of rewriting the items in between.

To avoid re-running the whole page, use a shard: a signal tracked inside a shard re-renders only that shard, not the entire page. The shard creates the signal, reads it, and hands the browser the handlers that change it:

```rust
# use topcoat::{Result, context::Cx, runtime::{shard, signal}, view::*};
# async fn load_page(_cx: &Cx, _page: f64) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn paginated(cx: &Cx) -> Result<impl View> {
    let page = signal(cx, || 1.0);
    let items = load_page(cx, page.get()).await?;

    Ok(view! {
        for item in items {
            <div>(item)</div>
        }

        <button @click=$(|_e| page.decrement())>"previous"</button>
        <button @click=$(|_e| page.increment())>"next"</button>
    })
}
```

Clicking a button changes `page` in the browser, and because the shard read it on the server, the shard runs again with the new value and swaps in the next page of items.

`.get_untracked()` and `.read_untracked()` read a signal's value without making anything depend on it, for a body that wants the value a run started with but should not run again when it changes.

[`Event`]: struct.Event.html
[`expr!`]: macro.expr.html
[`signal`]: fn.signal.html
[`view!`]: ../view/macro.view.html
[procedure]: attr.procedure.html
[shard]: attr.shard.html
