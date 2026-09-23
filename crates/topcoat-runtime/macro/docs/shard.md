A shard is a component that re-renders on the server when its inputs change in the browser. Its inputs include runtime [expression](macro.expr.html) arguments and signals read by the shard body. Each shard exposes an HTTP endpoint, so validate its inputs and check authorization in the shard.

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

# Calling a shard

Call a shard inside [`view!`] like a component. Each argument accepts a fixed value or a runtime expression:

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
    search_results(query: "shoes".to_owned())
})
# }
```

The initial page render includes the shard's content without an extra request.

When `query` changes, the browser sends the current arguments to the server and updates the shard with the returned HTML. Matching elements update in place to preserve focus and input state. Give each item in a reorderable list a stable `id` so the browser can match it after it moves.

Several signal changes in the same tick share one request. Starting a new request cancels any earlier request still in progress, so an older response cannot replace newer results.

# Shard state

Signals created inside a shard keep their values across re-renders. The browser sends their current values with each request, and [`signal`] resumes from them. Validate these values as user input.

Read a signal with `.get()` or `.read()` in the shard body to re-render the shard when it changes:

```rust
use topcoat::{Result, context::Cx, runtime::{shard, signal}, view::{View, view}};

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
# async fn load_page(_cx: &Cx, _page: usize) -> Result<Vec<String>> { Ok(vec![]) }
```

See the [runtime guide](../runtime/index.html#reading-signals-on-the-server) for server-side signal reads.

# Guards

A re-render request runs the shard function without its containing page or layouts. A shard that renders private data must check authorization itself, including whether the current user can access the data selected by its arguments:

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

# Arguments and return type

Argument types must be supported by [`expr!`]. Return a [`Result`] containing a [`View`].

A parameter named `cx` of type `&Cx` receives the server's request context. Omit it at the call site.

Pass a signal directly to a [`Signal<T>`] parameter to let the shard choose whether to track it. Calling `.get()` or `.read()` makes changes trigger a re-render. The untracked methods read its current value without triggering re-renders:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{shard, signal, Signal}};
# async fn search_products(_cx: &Cx, _query: &str, _limit: usize) -> Result<Vec<String>> { Ok(vec![]) }
#[shard]
async fn search_results(cx: &Cx, query: String, limit: Signal<usize>) -> Result<impl View> {
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
# let limit = signal(cx, || 10usize);
# Ok(view! {
search_results(query: $(query.get()), limit: limit)
# })
# }
```

# Registration

Register shards on the [`Router`] with `.discover()`, or mount one individually:

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
