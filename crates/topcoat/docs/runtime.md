Topcoat's runtime adds browser interactions to server-rendered pages. Write state and expressions in Rust alongside your markup. The runtime compiles the browser code to JavaScript and includes it with the page.

The runtime is **highly experimental**. Expressions support a limited set of Rust types and operations, and the API may change.

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

Call [`runtime()`](RouterBuilderRuntimeExt::runtime) to enable page reruns, and load the [asset bundle](../asset/index.html) to serve the browser script. Register your application layers before calling `.runtime()`:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
    runtime::RouterBuilderRuntimeExt,
};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .runtime()
        .build()
}
```

The runtime reruns a page by sending its current signal values to the page's URL. The [`RuntimeLayer`] added by `.runtime()` converts this request into a `GET`, so the page and its layouts can render with those values. Registering your layers first lets them handle the rerun as a `GET`. See [`RuntimeLayer`] for the request format and rewrite behavior.

Register your [procedures](#procedures) and [shards](#shards) separately. The example uses `.discover()` to find them.

# Runtime expressions

A `$(...)` block is a **runtime expression**. Use it in a view to render its value:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    <p>"The answer: " $(1 + 2)</p>
})
# }
```

The server evaluates the expression for the initial HTML. Expressions that need browser updates also compile to JavaScript and can run again without a server request.

Runtime expressions support a subset of Rust with matching behavior in both languages. See [`expr!`] for supported operations, captured values, and embedding JavaScript with `raw!`.

The expression `1 + 2` is static. To make an expression update, have it read a signal.

# Signals

A **signal** holds state for the browser. Create one with [`signal`], passing the request context and a closure for its initial value. Read it in a runtime expression with `.get()`:

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

The server computes the initial value and sends it to the browser. **Values sent back by the browser are user input and must be validated.**

A signal is cheap to clone. Runtime expressions can share it, and components can accept it as a `&Signal<T>` parameter.

In the browser, an expression runs again whenever a signal it reads changes. `.get()` reads the current value and `.set(...)` replaces it. Use an event handler to change `count` when the user clicks a button.

# Event handlers

An attribute starting with `@` attaches a DOM event handler, such as `@click`. Its value is a runtime expression returning a closure. The browser calls that closure when the event fires:

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

Clicking the button runs the closure, the closure updates the signal, and the `$(count.get())` text re-renders. The entire loop happens in the browser.

The closure receives an [`Event`] with access to the DOM event's data and methods. For example, an input handler can copy the element's value into a signal:

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

For operations outside the expression vocabulary, use a JavaScript string literal, such as `@click="alert('hi')"`.

Signals also have methods for common updates. For example, `$(|_e| count.increment())` adds one to a numeric signal.

# Bind attributes

An attribute starting with `:` is a **bind attribute**. The server renders its expression's initial value. The browser updates the attribute whenever a signal read by the expression changes:

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

Combine `:value` and `@input` to keep an input and signal in sync. The binding displays the signal's value, and the handler writes user edits back to it:

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

A **procedure** is an async server function called from a runtime expression. Use one when an event handler needs a server resource, such as a database:

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

The browser sends an HTTP request with the arguments and waits for the result. **Validate arguments and authorize access inside the procedure**, since callers can send their own requests. See [`#[procedure]`][procedure] for usage details.

# Shards

A **shard** is a component that re-renders on the server when its inputs change. Use one to update markup from server data, such as search results. Arguments accept fixed values or runtime expressions:

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

A shard has its own HTTP endpoint. **Validate its arguments and authorize access inside the shard**. Its page and layout guards do not run for requests to that endpoint. See [`#[shard]`][shard] for usage details.

# Reading signals on the server

Read a signal in server Rust with `.get()` to clone its value or `.read()` to borrow it. These are **tracked reads**. A change in the browser causes the page to re-render on the server with the new value:

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

The input updates in the browser while the server refreshes the product list. On re-render, [`signal`] restores the value sent by the browser. Reads inside `$(...)` update browser expressions and do not trigger a server render.

**Every value read on the server is user input and must not be trusted.** The client holds the signal and can send anything that fits its type, so validate the value before acting on it, exactly like a shard argument.

The browser updates existing elements in place to preserve focus, scroll position, and input state. Signals keep their values. Give items in a reorderable list stable IDs so each element follows its item.

To avoid re-running the whole page, use a shard: a signal tracked inside a shard re-renders only that shard, not the entire page. The shard creates the signal, reads it, and hands the browser the handlers that change it:

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

Clicking a button changes `page` in the browser, and because the shard read it on the server, the shard runs again with the new value and swaps in the next page of items.

A shard can also take a signal from its caller, through a parameter typed `Signal<T>` and passed directly as `signal`. The signal handle does not change when its value does, so whether a change re-renders the shard depends on how the shard body reads it. See [`#[shard]`][shard].

Use `.get_untracked()` or `.read_untracked()` to read a value on the server without making changes to it trigger another render.

[`Event`]: struct.Event.html
[`expr!`]: macro.expr.html
[`signal`]: fn.signal.html
[`view!`]: ../view/macro.view.html
[procedure]: attr.procedure.html
[shard]: attr.shard.html
