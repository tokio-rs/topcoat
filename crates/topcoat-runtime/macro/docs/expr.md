The [`expr!`] macro compiles a single Rust expression twice: into ordinary Rust that runs on the server during the initial render, and into equivalent JavaScript that ships with the page and re-runs in the browser. Inside a `view!` body a runtime expression is written `$(...)`, which lowers through [`expr!`]; the macro is rarely invoked directly.

Because every expression must behave identically in both languages, only a limited subset of Rust is supported: a small vocabulary of types and methods, and a restricted set of expression shapes. Both are listed below.

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal show = false;

    $(if show.get() { "shown" } else { "hidden" })
}
# }
```

The server evaluates the expression once to produce the initial HTML. In the browser, the compiled JavaScript re-runs whenever a signal it reads changes and patches the DOM in place. There is no wasm bundle, no client build step, and no server round-trip.

An invocation expands to an [`Expr`] value bundling the server-evaluated result with the JavaScript source.

# Captured variables

An identifier that is not defined inside the expression is captured from the surrounding Rust scope:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example(step: f64) -> Result {
view! {
    signal count = 0.0;

    $(count.get() + step)
}
# }
```

The captured value is serialized into the page during the render and becomes a constant in the generated JavaScript. It is a snapshot: the browser keeps the value from the render, and later changes on the server do not reach it. Captured values must belong to the shared vocabulary described next.

# The shared vocabulary

Expressions operate on a fixed vocabulary of types that exist on both sides, each exposing a subset of its Rust API:

- `f64`: arithmetic (`+`, `-`, `*`, `/`), comparisons, and negation. All numbers are `f64`, matching JavaScript; integer literals are not accepted, so write `1.0` rather than `1`.
- `bool`: `!`, comparisons, `then`, and `then_some`.
- `String` and `&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`, and comparisons.
- `Option<T>`: `is_some`, `is_none`, `unwrap`, and `expect`.
- `Result<T, E>`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, and `expect_err`.
- Tuples of vocabulary types.
- [`Signal`]: `get` and `set`.

# Supported syntax

Expressions use a subset of Rust's syntax:

- String, `f64`, and `bool` literals.
- The unary and binary operators listed above.
- Method calls, field access, and indexing.
- Blocks, with `let` bindings of plain identifiers; the trailing expression is the block's value.
- `if`/`else` as an expression.
- Closures, optionally `async`, and `.await`.
- `loop`, `while`, `break`, `continue`, and `return`.

Anything else -- `match`, integer literals, struct expressions, multi-segment paths -- is rejected with a compile error pointing at the unsupported expression.

# Embedding JavaScript

The `raw!` macro escapes to hand-written JavaScript for the parts of an expression the vocabulary does not cover. Its first argument is the JavaScript source; the optional second argument is the equivalent Rust, used when the server evaluates the expression:

```rust
# use topcoat::{Result, view::*};
# #[component]
# async fn example() -> Result {
view! {
    signal name = String::new();

    $({
        let n = name.get();
        raw!("${n}.toUpperCase()", n.to_uppercase())
    })
}
# }
```

`${ident}` inside the JavaScript string interpolates a binding from the expression's scope. Without the Rust argument the expression can no longer be evaluated on the server, so that form is only usable where the expression runs purely in the browser. In either form, keeping the two sides equivalent is up to you.

[`Expr`]: struct.Expr.html
[`Signal`]: struct.Signal.html
[`expr!`]: macro.expr.html
