A shard is a component that renders again on the server whenever its inputs change in the browser. Its arguments accept fixed values or runtime [expressions](macro.expr.html). The browser tracks the signals those expressions read, and when one changes, it requests a new render from the server and morphs the result into the page. A signal the shard body reads on the server counts as an input too. Each shard is an HTTP endpoint of your server, so its arguments **must not be trusted**.

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

Inside a [`view!`] body, call a shard like a component. Each parameter accepts a value of its declared type `T` or an [`Expr<T>`][`Expr`], such as a `$(...)` runtime expression:

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

During the page render, the shard runs inline like any component. The server evaluates each argument expression once and embeds the resulting view in the page. No extra request happens.

When the `query` signal changes, the browser sends the current argument values to the server, the shard function runs again, and the returned HTML is morphed into the shard's previous content. Elements that still exist are updated in place, so focus and what the user typed survive, and the rest of the page is left alone. Elements are matched by position and tag, and an `id` pins the match. Give each item of a list that can reorder an `id`, so the morph follows the item to its new position instead of rewriting the items in between.

Several signal changes in the same tick result in one request. Starting a request aborts any earlier one still in flight, so the latest arguments win.

# Shard State

A shard's content is a full view. The shard can create signals, attach event handlers, and contain nested shards. A re-render rebuilds that content from the server's HTML, but the signals created in the shard body keep their values. Their current values travel with every re-render request, and [`signal`] resumes from them instead of starting over. Those values are user input and **must not be trusted**, like the shard's arguments.

Reading one of those signals on the server with `.get()` or `.read()` makes the shard depend on it. A change in the browser then re-renders only that shard, not the entire page:

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

The [runtime guide](../runtime/index.html#reading-signals-on-the-server) covers reads on the server in full, including the untracked variants and what a read outside any shard does to the page.

# Guards

A shard has its own endpoint, and a request to it runs the shard function alone. Guards in the page or its layouts do not run, so a shard that renders private content must check authorization itself. The caller picks the argument values, so the check must cover them too: confirm that the current user may see the data the arguments select.

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

Argument types must belong to the shared vocabulary of [`expr!`], since their values travel between Rust and JavaScript. Each argument must be a plain identifier, not a pattern. The return type is a [`Result`] of a value implementing [`View`], like a component's.

A parameter named `cx` that borrows [`Cx`] is special. Just like in a component, the server fills it with the request context, and the call site does not pass it.

A signal can also be passed to a parameter of type [`Signal<T>`]. The argument is the signal itself, which does not change when its value does. The shard therefore only re-renders on a change if its body makes a tracked read of the signal. This lets a shard take an input that does not trigger a re-render by itself:

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

Each shard is served by its own route on the [`Router`]. `.discover()` registers every shard linked into the binary. You can also register shards one by one:

```rust
# use topcoat::{Result, router::Router, runtime::{shard, RouterBuilderShardExt}, view::{View, view}};
# #[shard]
# async fn search_results(query: String) -> Result<impl View> { Ok(view! { (query) }) }
let router = Router::builder().shard(search_results).build();
```

[`context`]: ../context/index.html
[`Expr`]: struct.Expr.html
[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Router`]: ../router/struct.Router.html
[`signal`]: fn.signal.html
[`Signal<T>`]: struct.Signal.html
[`expr!`]: macro.expr.html
[`view!`]: ../view/macro.view.html
[`View`]: ../view/trait.View.html
