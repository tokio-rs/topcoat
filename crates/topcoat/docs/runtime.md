Topcoat's runtime adds interactivity to server-rendered pages. Write expressions in [`view!`] to update the page when state changes. Topcoat checks them as Rust and generates the JavaScript for the browser.

The runtime is experimental, and its API may change. Expressions support a subset of Rust, documented in [`expr!`].

# Setup

Include `script()` in the document head to load the browser runtime:

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

Call [`runtime()`](RouterBuilderRuntimeExt::runtime) on the router builder and load the [asset bundle](../asset/index.html) that serves the script:

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

The example uses `.discover()` to register the application's endpoints. [Procedures](#procedures) and [shards](#shards) need this registration in addition to `.runtime()`.

# Runtime expressions

A **runtime expression**, written `$(...)`, computes a value to display in a view:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <p>"The answer: " $(1 + 2)</p>
})
# }
```

The server evaluates the expression for the initial HTML. If it reads state that can change, the browser updates its output when that state changes.

`$(...)` uses the [`expr!`] macro. See its reference for supported types, syntax, and captured values.

The expression above always displays `3`. To display a value that changes, read a signal.

# Signals

A **signal** holds state that can change in the browser. Create one with [`signal`] and read its value with `.get()`:

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

The server computes the initial value and sends it with the page. The browser then owns the value, so validate it if you read it back on the server. Cloning a signal shares its state. You can use it in several expressions or pass it to a component as `&Signal<T>`.

When a signal changes, expressions that read it update in the browser without a server request. Use `.set(...)` in an event handler to change its value.

# Event handlers

An attribute starting with `@` attaches a handler for a DOM event. Its value is a runtime expression containing a closure to run when the event fires:

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

Clicking the button updates `count` and the displayed text in the browser.

The closure receives an [`Event`]. For example, an input handler can copy `e.target.value` into a signal:

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

The handler can also be a JavaScript string literal, such as `@click="alert('hi')"`.

Some updates have a shorter form. For example, `count.increment()` adds one to `count`. See [`expr!`] for the available methods.

# Bind attributes

A **bind attribute**, prefixed with `:`, follows the value of a runtime expression. The server renders its initial value, and the browser updates it when a signal it reads changes:

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

Use a bind attribute and an event handler together to keep an input and a signal in sync. Here, `:value` displays the signal and `@input` updates it as the user types:

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

A **procedure** is an async server function that an event handler can call. Use one when an interaction needs server resources, such as a database:

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

The browser sends the arguments to the server and awaits the result. A procedure is a public HTTP endpoint, so validate its inputs and check authorization in the function. See [`#[procedure]`][procedure] for its requirements and error handling.

# Shards

A **shard** is a component that re-renders on the server when its inputs change. Pass runtime expressions as arguments to update part of a page, such as search results, from server data:

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

A shard body is ordinary server code. Re-render requests call its own HTTP endpoint, so validate its inputs and check authorization in the shard. See [`#[shard]`][shard] for details.

# Reading signals on the server

Reading a signal in server code makes the rendered content depend on it. `.get()` clones the current value and `.read()` borrows it. When the signal changes in the browser, the server re-renders the page with the new value:

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

Typing updates the signal in the browser and requests fresh search results from the server. On that request, [`signal`] uses the browser's value without running its initializer. Reads inside `$(...)` update in the browser and do not trigger a server render.

The browser can send any value that fits the signal's type. Validate it before using it on the server.

The browser updates matching elements in place to preserve focus, scroll position, and input state. Signals keep their values. Give each item in a reorderable list a stable `id` so the browser can match it after it moves.

To limit the server render to part of a page, read the signal inside a shard:

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

Clicking a button changes `page` and re-renders the shard with the new value.

A shard can also receive a `Signal<T>` from its caller. See [`#[shard]`][shard] for how the shard chooses whether to track it.

Use `.get_untracked()` or `.read_untracked()` to read the current value without causing a new render when it changes.

[`Event`]: struct.Event.html
[`expr!`]: macro.expr.html
[`signal`]: fn.signal.html
[`view!`]: ../view/macro.view.html
[procedure]: attr.procedure.html
[shard]: attr.shard.html
