# Validation

Validation errors are always exposed as the shared `props.errors` object. The key is reserved: declaring `errors` with [`Inertia::prop`](crate::Inertia::prop), application sharing, or request sharing returns a render error.

For a page rendered immediately, use [`Inertia::errors`](crate::Inertia::errors). Its value must serialize as an object.

For the common form workflow, validate a mutation, call [`flash_errors`](crate::flash_errors), and redirect with a 303 response. The next Inertia page receives the errors and consumes them.

```rust
# use serde_json::json;
# use topcoat_core::{context::Cx, error::Result};
# use topcoat_inertia::flash_errors;
fn validate(cx: &Cx) -> Result<()> {
    flash_errors(cx, json!({"email": "Choose a different address"}))
}
```

When the request includes `X-Inertia-Error-Bag`, transported errors are nested under that bag name. An immediate `.errors(...)` value has higher precedence and is not re-bagged.

Use the same persistent flash storage guidance as other one-time data. Large validation payloads should use a session or database-backed [`FlashStore`](crate::FlashStore), not the default cookie. Log internal validation-system failures through the application's normal production error path, and send only user-safe messages in the errors object.
