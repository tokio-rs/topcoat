A shard is a special type of component that can re-run whenever its arguments change in the browser. Arguments are runtime [expressions](macro.expr.html): the browser tracks the signals they read, and when one changes it requests a fresh render from the server and swaps the result into the DOM. Shards are exposed as API endpoints from your server; arguments **must not be trusted**.

```rust
use topcoat::{Result, context::Cx, runtime::shard, view::view};

#[shard]
async fn search_results(cx: &Cx, query: String) -> Result {
    let products = search_products(cx, &query).await?;
    view! {
        for product in products {
            <div>(product)</div>
        }
    }
}
# async fn search_products(_cx: &Cx, _query: &str) -> Result<Vec<String>> { Ok(vec![]) }
```

# Calling Shards

Inside a [`view!`] body, call a shard like a component, passing a runtime expression for each parameter:

```rust
# use topcoat::{Result, view::*, runtime::{shard, Event}};
# #[shard]
# async fn search_results(query: String) -> Result { view! { (query) } }
# #[component]
# async fn example() -> Result {
view! {
    signal query = String::new();

    <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

    search_results(query: $(query.get()))
}
# }
```

During the page render the shard runs inline like any component: the server evaluates each argument expression once and embeds the resulting view in the page. No extra request happens.

When the `query` signal changes, the current argument values are sent to the server, the shard function runs again, and the returned HTML replaces the shard's previous content in place; the rest of the page is untouched. Several signal changes in the same tick coalesce into one request, and starting a request aborts any earlier one still in flight, so the latest arguments win.

# WebSocket Shards

Write `#[shard(ws)]` when one connection should produce more than one render. WebSocket support is always available when Topcoat's router and runtime are enabled. The application must also depend on Tokio with its `sync` feature and on the stream trait it names, such as `futures-core`; add `async-stream` to construct the stream with its `stream!` macro.

A WebSocket shard takes exactly one non-`cx` parameter. Its declared type is `tokio::sync::mpsc::Receiver<Arg>`, while its component-facing property remains `Expr<Arg>`:

```rust
use topcoat::{Result, context::Cx, runtime::shard, view::view};

#[shard(ws)]
async fn search_results(
    cx: &Cx,
    queries: tokio::sync::mpsc::Receiver<String>,
) -> impl futures_core::Stream<Item = Result> {
    async_stream::stream! {
        let mut queries = queries;
        while let Some(query) = queries.recv().await {
            let products = search_products(cx, &query).await;
            yield view! {
                for product in products {
                    <div>(product)</div>
                }
            };
        }
    }
}
# async fn search_products(_cx: &Cx, _query: &str) -> Vec<String> { vec![] }
```

The browser sends the current argument when the socket opens and sends later values when the expression changes. Same-tick changes coalesce. Every successful stream item replaces the shard content, so the stream may also push renders without a new browser value. The input channel has capacity one. The socket closes when the stream errors or completes, the client disconnects, or the scope is disposed. An unexpected close is not reconnected automatically.

Server-side rendering and the hydrated connection are separate invocations. During server-side rendering, Topcoat creates a temporary capacity-one channel, seeds it with the initial `Arg`, drops its sender, invokes the shard, and uses the first stream item as the placeholder. During hydration, the browser opens a new persistent socket, Topcoat invokes the shard again, and the browser resends the current `Arg`. Code must not rely on state from the server-side invocation surviving into the socket invocation.

# Shard State

A shard's content is a full view: it can declare signals, attach event handlers, and contain nested shards. A re-render replaces that content wholesale, though, so state declared inside the shard -- like a `signal` in its `view!` -- resets each time. State that must survive re-renders lives outside the shard and flows in through its arguments.

# Arguments And Return Type

Argument types must belong to the shared vocabulary of [`expr!`], since their values cross between Rust and JavaScript. A regular shard returns [`Result`], whose `Ok` value is the rendered view.

A WebSocket shard must declare `impl Stream<Item = Output>`, and `Output` must be Topcoat's `Result<View>` (normally written `Result`). The receiver's `Arg` must belong to the same shared expression vocabulary. The macro checks the resolved receiver and stream types, not only their spelling, so similarly named types and a wrong stream item are rejected:

```rust,compile_fail
# use topcoat::runtime::shard;
mod tokio {
    pub mod sync {
        pub mod mpsc {
            pub struct Receiver<T>(pub std::marker::PhantomData<T>);
        }
    }
}

#[shard(ws)]
async fn invalid_receiver(
    values: tokio::sync::mpsc::Receiver<String>,
) -> impl futures_core::Stream<Item = topcoat::Result> {
    futures_util::stream::empty()
}
```

```rust,compile_fail
# use topcoat::runtime::shard;
#[shard(ws)]
async fn invalid_output(
    values: tokio::sync::mpsc::Receiver<String>,
) -> impl futures_core::Stream<Item = String> {
    futures_util::stream::empty()
}
```

A parameter named `cx` borrowing [`Cx`] is special: just like in a component, it is filled from the request context on the server and does not take an argument at the call site. For a WebSocket shard, the connection owns that request context for its full lifetime, including app and request context values established before the upgrade.

# Registration

Each shard is served by a route on the [`Router`]. `.discover()` registers every shard linked into the binary; alternatively, mount shards individually:

```rust
# use topcoat::{Result, router::Router, runtime::{shard, RouterBuilderShardExt}, view::view};
# #[shard]
# async fn search_results(query: String) -> Result { view! { (query) } }
let router = Router::builder().shard(search_results).build();
```

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Router`]: ../router/struct.Router.html
[`expr!`]: macro.expr.html
[`view!`]: ../view/macro.view.html
