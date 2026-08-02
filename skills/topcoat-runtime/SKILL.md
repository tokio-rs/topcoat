---
name: topcoat-runtime
description: Build, review, test, and debug client-side reactivity in Topcoat Rust applications. Use for signals, $(...) runtime expressions, event handlers, bind attributes, raw JavaScript escapes, procedures, shards, reactive server rendering, and client/server trust boundaries.
---

# Topcoat Runtime

Use Topcoat's runtime for targeted browser behavior without WebAssembly or a separate client build. Check the installed version and read `crates/topcoat/docs/runtime.md` plus the macro guides under `crates/topcoat-runtime/macro/docs/`.

## Choose the smallest mechanism

- Use a signal and `$(...)` when behavior can run locally in the browser.
- Use `#[procedure]` for an imperative async server call.
- Use `#[shard]` when changing client state should re-render a server component.
- Use ordinary forms or links when native navigation already fits.

```rust
view! {
    signal open = false;

    <button @click=$(|_event| open.toggle())>"Toggle"</button>
    <p :hidden=$(!open.get())>"Details"</p>
}
```

`@event=$(closure)` attaches a handler. `:attribute=$(expr)` updates an attribute when a signal it reads changes. Runtime expressions are evaluated as Rust for initial rendering and translated to JavaScript for browser updates.

## Respect the shared expression language

Use only the supported cross-language types, methods, operators, and syntax. Important constraints:

- Numbers are `f64`; write `1.0`, not `1`.
- Captured server values are serialized snapshots, not live server state.
- Unsupported Rust constructs fail at compile time.
- `raw!` embeds JavaScript; provide equivalent Rust when the expression also runs during server rendering.

Keep expressions small. Move server-only logic to a procedure or shard rather than expanding `raw!` usage.

## Call procedures safely

```rust
use topcoat::{Result, runtime::procedure};

#[procedure]
async fn save_name(cx: &Cx, name: String) -> Result<String> {
    let user = require_auth(cx).await?;
    Ok(update_name(cx, user, name).await?)
}
```

Call a procedure with `.await` inside browser-only async code such as an event closure. Its arguments and `Ok` value must use the shared expression vocabulary. A procedure call cannot run during server rendering.

Procedure arguments come from the client. Validate them and authenticate and authorize inside the procedure. If the browser must react to a domain failure, return that outcome as data; a procedure `Err` becomes an HTTP failure and is not observable as a value in the expression.

## Use shards for reactive server views

```rust
#[shard]
async fn results(cx: &Cx, query: String) -> Result {
    let user = require_auth(cx).await?;
    let rows = search(cx, user, &query).await?;
    view! { for row in rows { <p>(row.name)</p> } }
}
```

Call it with runtime arguments such as `results(query: $(query.get()))`. Initial rendering happens inline; later signal changes request new HTML and replace only the shard. Same-tick changes coalesce and a newer request aborts an older in-flight request.

State declared inside a shard resets when it is replaced. Keep persistent signals outside and pass their values in. Treat shard arguments as untrusted and authorize inside the shard because its endpoint does not run the page or layout guard.

Register procedures and shards with `.discover()` or their explicit router builder methods. Test initial HTML, generated behavior, endpoint authorization, invalid arguments, rapid updates, and state reset boundaries.
