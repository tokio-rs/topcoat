A procedure is an async server function that the browser can call from a runtime [expression](macro.expr.html). Use a procedure to run Rust code that runtime expressions do not support, or to reach server-only resources like the database. Each procedure is an HTTP endpoint of your server, so anyone can call it with any arguments. Its arguments **must not be trusted**.

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

The call sends the arguments to the server, runs the function there, and resolves to its return value. This takes a network round-trip, so a call has to be awaited in an async context, such as the body of an `async` closure.

A call only runs in the browser. The server type-checks it, but running it on the server panics. Place calls where the server render never runs them, like the closure body above.

# Arguments And Return Type

The argument types and the `Ok` type of the returned [`Result`] must belong to the shared vocabulary of [`expr!`], since their values travel between Rust and JavaScript.

A parameter named `cx` that borrows [`Cx`] is special. The server fills it with the request context, and it is not part of the arguments the browser sends:

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

Awaiting a call gives the procedure's `Ok` value directly. An `Err` becomes an error response, and the expression awaiting the call fails in the browser without a value. The expression cannot inspect the error. If the browser needs to react to failures, return the outcome as data instead, for example with an `Ok` type of `Result<String, String>`.

# Registration

Each procedure is served by its own route on the [`Router`]. `.discover()` registers every procedure linked into the binary. You can also register procedures one by one:

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
