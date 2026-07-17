Topcoat's runtime makes server-rendered pages interactive without a wasm bundle, a client build step, or a separate frontend. Reactive state and expressions are written inline in [`view!`], type-checked as ordinary Rust, and compiled to JavaScript that ships with the page.

The runtime is **highly experimental** and fairly limited today: expressions support only a small vocabulary of types and methods, and some patterns have no ergonomic answer yet. It will improve in future releases; expect both additions and breaking changes.

# Setup

Interactive pages need the runtime's browser script. `script()` renders the script tag; include it in your document head:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn document() -> Result {
view! {
    <html>
        <head>
            topcoat::runtime::script()
        </head>
        <body></body>
    </html>
}
# }
```

The script is served as a Topcoat [asset](../asset/index.html), so the asset bundle must be loaded on the router:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

pub fn router() -> Router {
    Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build()
}
```

`.discover()` also registers the server endpoints behind [procedures](#procedures) and [shards](#shards), covered later in this guide.

# Signals

A **signal** is a piece of state that lives in the browser. Declare one with a `signal` statement inside a [`view!`] body:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal count = 0.0;
}
# }
```

The initial value is an ordinary Rust expression, evaluated once during the server render and serialized into the page; the browser picks it up as reactive state.

On its own a signal does nothing. Its value is read with `.get()` and replaced with `.set(...)`, but only inside runtime expressions, introduced next; the rest of the view's Rust code cannot touch it. When a signal changes in the browser, everything that read it updates automatically.

# Runtime expressions

A **runtime expression** is written `$(...)` and can stand wherever a view node can:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal count = 0.0;

    <p>"Count: " $(count.get())</p>
}
# }
```

The expression is type-checked Rust, but it is compiled twice: the server evaluates it once for the initial HTML, and an equivalent JavaScript translation ships with the page. In the browser it re-runs whenever a signal it reads changes -- the text above updates the moment `count` does, with no server round-trip. Nothing changes `count` yet; that is what event handlers are for.

Because a runtime expression must behave identically in both languages, only a subset of Rust is supported: a small vocabulary of types and methods. The [`expr!`] macro documents that vocabulary, how captured variables behave, and the `raw!` escape hatch to hand-written JavaScript.

# Event handlers

An attribute starting with `@` attaches an event handler: `@click`, `@input`, or any other DOM event name. Its value is a runtime expression evaluating to a closure, which runs in the browser each time the event fires. Handlers are where signals change:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal count = 0.0;

    <button @click=$(|_e| count.set(count.get() + 1.0))>"+1"</button>
    <p>"Count: " $(count.get())</p>
}
# }
```

Clicking the button runs the closure, the closure updates the signal, and the `$(count.get())` text re-renders. The entire loop happens in the browser.

The closure receives an [`Event`] mirroring the DOM event: fields like `e.target.value`, `e.key`, and `e.client_x`, and methods like `e.prevent_default()`. A typical input handler forwards the element's value into a signal:

```rust
# use topcoat::{Result, view::*, runtime::Event};
# #[component]
# async fn example() -> Result {
view! {
    signal query = String::new();

    <input @input=$(|e: Event| query.set(e.target.value))>
}
# }
```

For the rare event logic the expression vocabulary cannot say, the value can also be a string literal of raw JavaScript: `@click="alert('hi')"`.

# Bind attributes

An attribute starting with `:` is a **bind attribute**: its value is a runtime expression, and the attribute is kept in sync with it. The server renders the initial value like a normal attribute; the browser re-applies it whenever a signal the expression reads changes:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal open = false;

    <button @click=$(|_e| open.set(!open.get()))>"What is Topcoat?"</button>
    <p :hidden=$(!open.get())>"A fullstack Rust framework."</p>
}
# }
```

A `bool` expression toggles the attribute's presence, like `hidden` above. Other values set the attribute's value; form state like `value` and `checked` is applied as the live DOM property, so `:value` on an input does what you expect.

# Procedures

Runtime expressions run in the browser, so they cannot query the database or use Rust beyond the shared vocabulary. When an event handler needs the server, it calls a **procedure**: an async server function invoked from a runtime expression like any other async function:

```rust
# use topcoat::{Result, view::*, runtime::procedure};
#[procedure]
async fn double(value: f64) -> Result<f64> {
    Ok(value * 2.0)
}

# #[component]
# async fn example() -> Result {
view! {
    signal count = 1.0;

    <button @click=$(async |_e| {
        let doubled = double(count.get()).await;
        count.set(doubled);
    })>
        "double it"
    </button>
}
# }
```

The call is an HTTP request under the hood: the arguments travel to the server, the function runs there, and the `.await` resolves to its result. See [`#[procedure]`][procedure] for the details: argument and return types, the `cx` parameter, error handling, and registration.

# Shards

When it is the markup itself that needs the server -- fresh search results as the user types -- use a **shard**: a component that re-renders on the server whenever one of its arguments changes. Arguments are runtime expressions; the browser sends their current values to the server and swaps the returned HTML in place:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{shard, Event}};
# async fn search_products(_cx: &Cx, _query: &str) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn search_results(cx: &Cx, query: String) -> Result {
    let products = search_products(cx, &query).await?;
    view! {
        for product in products {
            <div>(product)</div>
        }
    }
}

# #[component]
# async fn example() -> Result {
view! {
    signal query = String::new();

    <input @input=$(|e: Event| query.set(e.target.value))>

    search_results(query: $(query.get()))
}
# }
```

A shard body is ordinary server code, like any component. See [`#[shard]`][shard] for the details: how re-renders behave, shard state, and registration.

[`Event`]: struct.Event.html
[`expr!`]: macro.expr.html
[`view!`]: ../view/macro.view.html
[procedure]: attr.procedure.html
[shard]: attr.shard.html
