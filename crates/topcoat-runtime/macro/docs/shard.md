A shard is a special type of component that can re-run whenever its inputs change in the browser. Arguments are runtime [expressions](macro.expr.html): the browser tracks the signals they read, and when one changes it requests a fresh render from the server and swaps the result into the DOM. A signal the shard body reads on the server counts as an input too. Shards are exposed as API endpoints from your server; arguments **must not be trusted**.

```rust
use topcoat::{Result, context::Cx, runtime::shard, view::{View, view}};

#[shard]
async fn search_results(cx: &Cx, query: String) -> Result<impl View> {
    let products = search_products(cx, &query).await?;
    Ok(view! {
        for product in products {
            <div>(product)</div>
        }
    })
}
# async fn search_products(_cx: &Cx, _query: &str) -> Result<Vec<String>> { Ok(vec![]) }
```

# Calling Shards

Inside a [`view!`] body, call a shard like a component, passing a runtime expression for each parameter:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{shard, signal, Event}};
# #[shard]
# async fn search_results(query: String) -> Result<impl View> { Ok(view! { (query) }) }
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let query = signal(cx, String::new);

Ok(view! {
    <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

    search_results(query: $(query.get()))
})
# }
```

During the page render the shard runs inline like any component: the server evaluates each argument expression once and embeds the resulting view in the page. No extra request happens.

When the `query` signal changes, the current argument values are sent to the server, the shard function runs again, and the returned HTML is morphed into the shard's previous content: elements that still exist are updated in place, so focus and what the user typed survive, and the rest of the page is untouched. Elements are matched by position and tag, and an `id` pins the match, so give the items of a list that can reorder an `id` and the morph follows each item to its new position instead of rewriting the items in between. Several signal changes in the same tick coalesce into one request, and starting a request aborts any earlier one still in flight, so the latest arguments win.

# Shard State

A shard's content is a full view: the shard can create signals, attach event handlers, and contain nested shards. A re-render rebuilds that content from the server's HTML, but the signals created in the shard body keep their values: their current values travel with every re-render request, and [`signal`] resumes from them instead of starting over. Those values are user input and **must not be trusted**, like the shard's arguments.

Reading one of those signals on the server with `.get()` or `.read()` makes the shard depend on it, so a change in the browser re-renders only that shard, not the entire page:

```rust
use topcoat::{Result, context::Cx, runtime::{shard, signal}, view::{View, view}};

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
# async fn load_page(_cx: &Cx, _page: f64) -> Result<Vec<String>> { Ok(vec![]) }
```

The [runtime guide](../runtime/index.html#reading-signals-on-the-server) covers server-side reads in full, including the untracked variants and what a read outside any shard does to the page.

# Guards

A shard has its own endpoint, and a request to it runs the shard function alone. Guards applied by the page or its layouts never run, so a shard that renders private content resolves authorization itself. The caller picks the argument values, so that check covers them too: confirm the current user may see the data the arguments select.

```rust
use topcoat::{Result, context::Cx, runtime::shard, view::{View, view}};

#[shard]
async fn ledger_rows(cx: &Cx, account: String) -> Result<impl View> {
    let user = require_auth(cx).await?;
    let rows = fetch_rows(cx, &user, &account).await?;

    Ok(view! {
        for row in rows {
            <div>(row)</div>
        }
    })
}
# struct User;
# async fn require_auth(_cx: &Cx) -> Result<User> { Ok(User) }
# async fn fetch_rows(_cx: &Cx, _user: &User, _account: &str) -> Result<Vec<String>> { Ok(vec![]) }
```

The [`context`] module covers writing guards as functions on [`Cx`].

# Arguments And Return Type

Argument types must belong to the shared vocabulary of [`expr!`], since their values cross between Rust and JavaScript. The return type is a [`Result`] of a value implementing [`View`], like a component's.

A parameter named `cx` borrowing [`Cx`] is special: just like in a component, it is filled from the request context on the server and does not take an argument at the call site.

A signal can also be passed as an argument, to a parameter typed [`Signal<T>`]. The argument is the signal handle, which does not change when its value does, so the shard does not re-render on a change unless its body reads the signal tracked. This is how to pass an input the shard does not track directly:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{shard, signal, Signal}};
# async fn search_products(_cx: &Cx, _query: &str, _limit: f64) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn search_results(cx: &Cx, query: String, limit: Signal<f64>) -> Result<impl View> {
    // A new limit takes effect on the next re-render, but does not cause one.
    let products = search_products(cx, &query, limit.get_untracked()).await?;

    Ok(view! {
        for product in products {
            <div>(product)</div>
        }
    })
}

# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
# let query = signal(cx, String::new);
# let limit = signal(cx, || 10.0);
# Ok(view! {
search_results(query: $(query.get()), limit: $(limit))
# })
# }
```

# Registration

Each shard is served by a route on the [`Router`]. `.discover()` registers every shard linked into the binary; alternatively, mount shards individually:

```rust
# use topcoat::{Result, router::Router, runtime::{shard, RouterBuilderShardExt}, view::{View, view}};
# #[shard]
# async fn search_results(query: String) -> Result<impl View> { Ok(view! { (query) }) }
let router = Router::builder().shard(search_results).build();
```

[`context`]: ../context/index.html
[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Router`]: ../router/struct.Router.html
[`signal`]: fn.signal.html
[`Signal<T>`]: struct.Signal.html
[`expr!`]: macro.expr.html
[`view!`]: ../view/macro.view.html
[`View`]: ../view/trait.View.html
