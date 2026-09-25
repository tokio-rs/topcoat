A shard is a component that re-renders on the server when its inputs change in the browser. Pass fixed values or runtime [expressions](macro.expr.html) as arguments. Changes to signals read by those expressions, or by the shard body, trigger a render. Each shard has an HTTP endpoint, so **validate its inputs and authorize access inside the shard**.

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

Inside a [`view!`] body, call a shard like a component. Each parameter accepts its declared type `T` or an `Expr<T>`, with conversion handled automatically:

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

The initial page render includes the shard's content. It does not need a separate request. Live content inside a shard, such as a [`live!`] region, streams its updates with the page on that render and with the shard's own response on a re-render.

When `query` changes, the browser sends the current arguments to the server and updates the shard with the returned HTML. Existing elements are updated in place to preserve focus and input state. The rest of the page is unchanged.

Elements are matched by position and tag, or by `id` when present. Give items in a reorderable list stable IDs so each element follows its item. Signal changes in the same tick share one request. A new request aborts any earlier pending request, so the latest arguments win.

# Shard State

A shard supports the same content as a component. Signals created in its body keep their values across re-renders. The browser sends those values with each request, and [`signal`] restores them. **Treat restored values as user input**, just like shard arguments.

Reading one of those signals on the server with `.get()` or `.read()` makes the shard depend on it, so a change in the browser re-renders only that shard, not the entire page:

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

See [Reading signals on the server](../runtime/index.html#reading-signals-on-the-server) for tracking rules.

# Guards

A request to a shard's endpoint runs the shard without its page or layouts. Their guards do not protect it. Check authorization inside the shard, including whether the user may access the data selected by its arguments.

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

A parameter named `cx` of type `&Cx` receives the server request context. Callers omit this argument.

A parameter typed [`Signal<T>`] accepts a signal handle. Passing the handle alone does not make the shard depend on its value. A tracked read in the shard body creates that dependency. Use an untracked read to use the value without triggering renders when it changes:

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

Register a shard on the [`Router`] so the browser can request new content. It implements [`Route`], so pass its name to `.route()`:

```rust
# use topcoat::{Result, router::Router, runtime::shard, view::{View, view}};
# #[shard]
# async fn search_results(query: String) -> Result<impl View> { Ok(view! { (query) }) }
let router = Router::builder().route(search_results).build();
```

With the `discover` feature enabled, `.discover()` registers all shards linked into the application.

# Path

Topcoat generates an internal path for each shard. This path can change between builds. To choose a stable endpoint, pass an absolute path to `#[shard]`:

```rust
use topcoat::{Result, runtime::shard, view::{View, view}};

#[shard("/search/results")]
async fn search_results(query: String) -> Result<impl View> {
    Ok(view! { <p>"Results for " (query)</p> })
}
```

The browser re-renders this shard with a `POST` request to `/search/results`. Arguments and signal values are sent in the request body, so the path cannot contain dynamic or catch-all parameters.

The path follows the router's [path syntax](../router/index.html#paths). Groups affect layer matching but are omitted from the URL. For example, `#[shard("/(api)/search/results")]` serves re-render requests at `/search/results`.

[`context`]: ../context/index.html
[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Route`]: ../router/trait.Route.html
[`Router`]: ../router/struct.Router.html
[`signal`]: fn.signal.html
[`Signal<T>`]: struct.Signal.html
[`expr!`]: macro.expr.html
[`live!`]: ../view/macro.live.html
[`view!`]: ../view/macro.view.html
[`View`]: ../view/trait.View.html
