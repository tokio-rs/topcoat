A shard is a special type of component that can re-run whenever its arguments change in the browser. Arguments are runtime [expressions](macro.expr.html): the browser tracks the signals they read, and when one changes it requests a fresh render from the server and swaps the result into the DOM. Shards are exposed as API endpoints from your server; arguments **must not be trused**.

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

# Shard State

A shard's content is a full view: it can declare signals, attach event handlers, and contain nested shards. A re-render replaces that content wholesale, though, so state declared inside the shard -- like a `signal` in its `view!` -- resets each time. State that must survive re-renders lives outside the shard and flows in through its arguments.

# Arguments And Return Type

Argument types must belong to the shared vocabulary of [`expr!`], since their values cross between Rust and JavaScript. The return type is [`Result`], whose `Ok` value is the rendered view.

A parameter named `cx` borrowing [`Cx`] is special: just like in a component, it is filled from the request context on the server and does not take an argument at the call site.

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
