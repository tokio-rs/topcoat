A procedure is an async server function called from a runtime [expression](macro.expr.html). Use it when browser code needs server resources, such as a database. Each procedure has an HTTP endpoint. **Validate its arguments and authorize access inside the procedure**, since callers can send their own requests.

```rust
use topcoat::{Result, runtime::procedure};

#[procedure]
async fn double(value: usize) -> Result<usize> {
    Ok(value * 2)
}
```

# Calling Procedures

Inside a runtime expression, call a procedure like an ordinary async function and `.await` its result:

```rust
# use topcoat::{Result, context::Cx, view::*, runtime::{procedure, signal}};
# #[procedure]
# async fn double(value: usize) -> Result<usize> { Ok(value * 2) }
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let count = signal(cx, || 1usize);

Ok(view! {
    <button
        @click=$(async |_e| {
            let doubled = double(count.get()).await;
            count.set(doubled);
        })
    >
        "double it"
    </button>

    $(count.get())
})
# }
```

The call sends its arguments to the server and waits for the function's result. Await it in an async context, such as an event handler's `async` closure.

A procedure call in a runtime expression must run only in the browser. The server type-checks the call but panics if it evaluates it during rendering. An event-handler closure, as above, defers the call until a browser event.

# Arguments And Return Type

Argument types and the `Ok` type of the returned [`Result`] must belong to the shared vocabulary of [`expr!`], since their values cross between Rust and JavaScript.

A parameter named `cx` of type `&Cx` receives the server request context. Callers omit this argument:

```rust
use topcoat::{Result, context::Cx, runtime::procedure};

#[procedure]
async fn search(cx: &Cx, query: String) -> Result<String> {
    // Query the database, read app context, check the session, ...
#   let _ = cx;
    Ok(query)
}
```

# Errors

Awaiting a call yields the procedure's `Ok` value. If the procedure returns `Err`, the server sends an error response and the browser expression fails. The expression cannot inspect that error. To let the client handle a failure, return it as data, for example with an `Ok` type of `Result<String, String>`.

# Registration

Register a procedure on the [`Router`] before calling it from the browser. It implements [`Route`], so pass its name to `.route()`:

```rust
# use topcoat::{Result, router::Router, runtime::procedure};
# #[procedure]
# async fn double(value: usize) -> Result<usize> { Ok(value * 2) }
let router = Router::builder().route(double).build();
```

With the `discover` feature enabled, `.discover()` registers all procedures linked into the application.

# Path

Topcoat generates an internal path for each procedure. This path can change between builds. To choose a stable endpoint, pass an absolute path to `#[procedure]`:

```rust
use topcoat::{Result, runtime::procedure};

#[procedure("/api/double")]
async fn double(value: usize) -> Result<usize> {
    Ok(value * 2)
}
```

The browser calls this procedure with a `POST` request to `/api/double`. Arguments are sent in the request body, so the path cannot contain dynamic or catch-all parameters.

The path follows the router's [path syntax](../router/index.html#paths). Groups affect layer matching but are omitted from the URL. For example, `#[procedure("/(api)/double")]` serves calls at `/double`.

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Route`]: ../router/trait.Route.html
[`Router`]: ../router/struct.Router.html
[`expr!`]: macro.expr.html
