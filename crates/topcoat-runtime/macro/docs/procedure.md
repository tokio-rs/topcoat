A procedure is an async server function callable from a runtime [expression](macro.expr.html). Use it to access server resources, such as a database. Each procedure exposes an HTTP endpoint, so validate its arguments and check authorization in the function.

```rust
use topcoat::{Result, runtime::procedure};

#[procedure]
async fn double(value: usize) -> Result<usize> {
    Ok(value * 2)
}
```

# Calling a procedure

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

The browser sends the arguments to the server and awaits the result. Make the call inside an `async` closure.

Calling a procedure during server rendering panics. Place calls in browser-only code, such as the event handler above.

# Arguments and return type

Argument types and the `Ok` type of the returned [`Result`] must be supported by [`expr!`].

A parameter named `cx` of type `&Cx` receives the server's request context. Omit it when calling the procedure:

```rust
use topcoat::{Result, context::Cx, runtime::procedure};

#[procedure]
async fn search(cx: &Cx, query: String) -> Result<String> {
    // Use cx to check access before reading server data.
#   let _ = cx;
    Ok(query)
}
```

# Errors

Awaiting a call yields its `Ok` value. If the procedure returns `Err`, the server sends an error response and the browser expression fails. The expression cannot inspect that error. To let the browser handle a failure, return it as data, for example by making the outer `Ok` type `Result<String, String>`.

# Registration

Register procedures on the [`Router`] with `.discover()`, or mount one individually:

```rust
# use topcoat::{Result, router::Router, runtime::{procedure, RouterBuilderProcedureExt}};
# #[procedure]
# async fn double(value: usize) -> Result<usize> { Ok(value * 2) }
let router = Router::builder().procedure(double).build();
```

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Router`]: ../router/struct.Router.html
[`expr!`]: macro.expr.html
