A procedure is an async server function that the browser can call from inside a runtime [expression](macro.expr.html). Use procedures to run more complex Rust codes that are not supported by runtime expressions, or to use server-only resources like the database. Procedures are exposed as HTTP API endpoints from your server; parameters can be spoofed and **must not be trusted**.

```rust
use topcoat::{Result, runtime::procedure};

#[procedure]
async fn double(value: f64) -> Result<f64> {
    Ok(value * 2.0)
}
```

# Calling Procedures

Inside a runtime expression, call a procedure like an ordinary async function and `.await` its result:

```rust
# use topcoat::{Result, view::*, runtime::procedure};
# #[procedure]
# async fn double(value: f64) -> Result<f64> { Ok(value * 2.0) }
# #[component]
# async fn example() -> Result {
view! {
    signal count = 1.0;

    <button
        @click=$(async |_e| {
            let doubled = double(count.get()).await;
            count.set(doubled);
        })
    >
        "double it"
    </button>

    $(count.get())
}
# }
```

The call sends the arguments to the server, runs the function there, and resolves to its return value. Since that takes a network round-trip, calls are only possible in an async position, such as the body of an `async` closure.

A procedure call never executes during the server render: the server type-checks the call but the request only happens in the browser. Procedure calls can therefore only appear where the expression never runs server-side, like the closure bodies above.

# Arguments And Return Type

Argument types and the `Ok` type of the returned [`Result`] must belong to the shared vocabulary of [`expr!`], since their values cross between Rust and JavaScript.

A parameter named `cx` borrowing [`Cx`] is special: it is filled from the request context on the server and is not part of the call signature the client sees:

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

Awaiting a call yields the procedure's `Ok` value directly. An `Err` becomes an error response, and the expression awaiting the call fails in the browser without a value; the error itself is not observable from the expression. If the client needs to react to failures, return the outcome as data instead, for example with an `Ok` type of `Result<String, String>`.

# Registration

Each procedure is served by a route on the [`Router`]. `.discover()` registers every procedure linked into the binary; alternatively, mount procedures individually:

```rust
# use topcoat::{Result, router::Router, runtime::{procedure, RouterBuilderProcedureExt}};
# #[procedure]
# async fn double(value: f64) -> Result<f64> { Ok(value * 2.0) }
let router = Router::builder().procedure(double).build();
```

[`Cx`]: ../context/struct.Cx.html
[`Result`]: ../type.Result.html
[`Router`]: ../router/struct.Router.html
[`expr!`]: macro.expr.html
