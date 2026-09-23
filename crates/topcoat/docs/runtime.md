Topcoat's runtime makes server-rendered pages interactive without a wasm bundle, a client build step, or a separate frontend. You write reactive state and expressions inline in [`view!`]. They are type-checked as ordinary Rust and compiled to JavaScript that ships with the page.

The runtime is **highly experimental** and still limited. Expressions support only a small vocabulary of types and methods, and many patterns have no ergonomic answer yet. Expect both additions and breaking changes in future releases.

# Setup

Interactive pages need the runtime's browser script. The `script` component renders its `<script>` tag. Include it in your document head:

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

The router needs two things as well. [`runtime()`](RouterBuilderRuntimeExt::runtime) mounts the routes the browser script calls on its own. The script itself is served as a Topcoat [asset](../asset/index.html), so the asset bundle must be loaded too:

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

`.runtime()` covers only the runtime's own routes. The endpoints behind your [procedures](#procedures) and [shards](#shards), covered later in this guide, come from annotated items that `.discover()` registers like pages and layouts. Rendering `script` on a router built without `.runtime()` panics.

# Runtime expressions

A `$(...)` block is a **runtime expression** and can stand wherever a view node can:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <p>"The answer: " $(1 + 2)</p>
})
# }
```

The expression is type-checked Rust, but it is compiled twice. The server evaluates it once for the initial HTML. An equivalent JavaScript translation ships with the page, where it can run again without any help from the server.

A runtime expression must behave the same in both languages, so only a subset of Rust is supported: a small vocabulary of types and methods. `$(...)` is shorthand for the [`expr!`] macro, whose documentation lists that vocabulary and explains how captured variables behave and how to embed hand-written JavaScript with `raw!`.

The browser has no reason to run `1 + 2` a second time, since the answer is always `3`. Expressions become useful when they read state that changes: signals.

# Signals

A **signal** is a piece of state that lives in the browser. Create one with [`signal`], passing the request context and a closure producing the initial value, and read it in a runtime expression with `.get()`:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 0usize);

Ok(view! {
    <p>"Count: " $(count.get())</p>
})
# }
```

The initial value is computed once during the server render and serialized into the page, where the browser picks it up as reactive state. **From then on the value is user input.** The browser holds it, so anything the server later reads back from the signal **must not be trusted**.

A signal belongs to the page, layout, component, or shard body that creates it. It is an ordinary value that is cheap to clone. Any number of runtime expressions in that body's view can capture it, and the body can pass it down to the components it renders as `&Signal<T>`.

In the browser, a runtime expression re-runs whenever a signal it read changes. The text above updates the moment `count` does, with no server round-trip. Inside an expression, `.get()` reads the signal's current value and `.set(...)` replaces it. Nothing changes `count` yet, though. That is what event handlers are for.

# Event handlers

An attribute starting with `@` attaches an event handler: `@click`, `@input`, or any other DOM event name. Its value is a runtime expression that evaluates to a closure. The closure runs in the browser each time the event fires, and it is where signals change:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 0usize);

Ok(view! {
    <button @click=$(|_e| count.set(count.get() + 1))>"+1"</button>
    <p>"Count: " $(count.get())</p>
})
# }
```

Clicking the button runs the closure, the closure updates the signal, and the `$(count.get())` text re-renders. All of this happens in the browser.

The closure receives an [`Event`] that mirrors the DOM event, with fields like `e.target.value`, `e.key`, and `e.client_x`, and methods like `e.prevent_default()`. A typical input handler copies the element's value into a signal:

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

When a handler needs something the expression vocabulary cannot express, the value can instead be a string literal of JavaScript. The string must evaluate to the handler function, for example `@click="() => alert('hi')"`.

`set` is the general write. A few updates that depend on the current value have a shorter form: `toggle` on a `bool` signal, `increment` and `decrement` on a numeric signal, and `push_str` on a `String` signal. The counter's handler can be written as `$(|_e| count.increment())`.

# Bind attributes

An attribute starting with `:` is a **bind attribute**. Its value is a runtime expression, and the attribute is kept in sync with it. The server renders the initial value like a normal attribute, and the browser applies it again whenever a signal the expression reads changes:

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

A bind attribute and an event handler together sync an element and a signal in both directions. `:value` keeps the input showing the signal, and `@input` writes every keystroke back into it:

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

Runtime expressions run in the browser, so they cannot query the database or use Rust beyond the shared vocabulary. When an event handler needs the server, it calls a **procedure**: an async server function that a runtime expression calls like any other async function:

```rust
# use topcoat::{Result, context::Cx, runtime::{procedure, signal}, view::*};
#[procedure]
async fn double(value: usize) -> Result<usize> {
    Ok(value * 2)
}

# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 1usize);

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

The call is an HTTP request. The arguments travel to the server, the function runs there, and the `.await` resolves to its result. This also means every procedure is an API endpoint of your server. Anyone can call it with any arguments, so its inputs **must not be trusted**. See [`#[procedure]`][procedure] for argument and return types, the `cx` parameter, error handling, and registration.

# Shards

When the markup itself needs the server, such as search results that update as the user types, use a **shard**: a component that re-renders on the server whenever one of its arguments changes. Arguments accept fixed values or runtime expressions. The browser sends their current values to the server and morphs the returned HTML into the page:

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

A shard body is ordinary server code, like any component. Its re-renders are served by an API endpoint of your server, so a shard's arguments can be forged just like a procedure's and **must not be trusted**. See [`#[shard]`][shard] for how re-renders behave, shard state, guards, and registration.

# Reading signals on the server

A signal can also be read in plain Rust, outside any runtime expression. `.get()` clones the current value and `.read()` borrows it. Both are **tracked reads**: they make the page depend on the signal. When the signal changes in the browser, the page runs again on the server with the signal's current value, and the result replaces the page's content:

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

The input keeps working as a browser-only binding, and the product list follows it through the server. On the re-run, [`signal`] starts from the value the browser sent instead of computing a new one, so the page picks up where the browser left off. Reads inside a `$(...)` expression never make the page depend on a signal. They are the browser-side path and stay in the browser.

**Every value read on the server is user input and must not be trusted.** The browser holds the signal and can send anything that fits its type, so validate the value before acting on it, exactly like a shard argument.

The result is morphed into the page rather than swapped in. Elements that still exist are updated in place, so focus, scroll position, and what the user is typing survive a re-run, and every signal keeps its value. Give each item of a list that can reorder an `id`, so the morph follows the item to its new position instead of rewriting the items in between.

To avoid re-running the whole page, use a shard. A tracked read inside a shard re-renders only that shard. Here the shard creates the signal, reads it, and renders the handlers that change it:

```rust
# use topcoat::{Result, context::Cx, runtime::{shard, signal}, view::*};
# async fn load_page(_cx: &Cx, _page: usize) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn paginated(cx: &Cx) -> Result<impl View> {
    let page = signal(cx, || 1usize);
    let items = load_page(cx, page.get()).await?;

    Ok(view! {
        for item in items {
            <div>(item)</div>
        }

        <button @click=$(|_e| {
            if page.get() > 1 {
                page.decrement()
            }
        })>"previous"</button>
        <button @click=$(|_e| page.increment())>"next"</button>
    })
}
```

Clicking a button changes `page` in the browser. Because the shard read it on the server, the shard runs again with the new value and renders the next page of items.

A shard can also take a signal from its caller, through a parameter of type `Signal<T>`. The caller passes the signal itself, as in `limit: limit`. The signal does not change when its value does, so whether a change re-renders the shard depends on how the shard body reads it. See [`#[shard]`][shard].

`.get_untracked()` and `.read_untracked()` read a signal's value without making anything depend on it. Use them when a body needs the current value but should not run again when it changes.

[`Event`]: struct.Event.html
[`expr!`]: macro.expr.html
[`signal`]: fn.signal.html
[`view!`]: ../view/macro.view.html
[procedure]: attr.procedure.html
[shard]: attr.shard.html
