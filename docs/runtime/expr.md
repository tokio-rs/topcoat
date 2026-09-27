The [`expr!`] macro defines an expression that can run on the server and in the browser. It compiles to Rust for the initial render and JavaScript for browser updates. Inside `view!`, write runtime expressions as `$(...)`.

Expressions support a subset of Rust with matching behavior in both languages. The supported types, operations, and syntax are described below.

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let show = signal(cx, || false);

Ok(view! {
    $(if show.get() { "shown" } else { "hidden" })
})
# }
```

The server evaluates the expression for the initial HTML. In the browser, it runs again whenever a signal it reads changes and updates the DOM without a server request.

The macro returns an [`Expr`] containing the server result and browser code.

Server evaluation is synchronous. An expression that reads no signals renders as static content. An event-handler closure's body runs only when the browser invokes it.

# Captured variables

An identifier that is not defined inside the expression is captured from the surrounding Rust scope:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx, step: usize) -> Result<impl View> {
let count = signal(cx, || 0usize);

Ok(view! {
    $(count.get() + step)
})
# }
```

Capturing clones the value, so surrounding code can still use it. The browser receives a snapshot from the render. Later server changes do not update that snapshot. Captured values must use the supported types described below. Capturing an owned collection clones its elements. Capturing a slice borrows its elements on the server, but still sends a snapshot to the browser.

A captured `Expr<T>` behaves as `T` inside another expression. The server reuses its evaluated value. The browser evaluates its expression at each use, so signal reads remain reactive, including inside event handlers.

```rust
# use topcoat::{Result, context::Cx, runtime::{expr, signal}, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let selected = signal(cx, || "overview".to_owned());
let active = expr!(selected.get() == "overview");
let label = expr!(if active { "Selected" } else { "Select" });
# Ok(view! { (label) })
# }
```

# The shared vocabulary

Runtime types expose a subset of their Rust APIs:

- `f64`: arithmetic (`+`, `-`, `*`, `/`), comparisons, and negation. Floating-point literals are `f64`. Text output follows Rust's `Display`, including decimal notation and the spellings `inf`, `-inf`, and `-0`.
- Rust integer types: arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons, and negation for signed types. Unsuffixed integer literals are `usize`; use a suffix for another type, such as `42u64` or `-1i32`. Operands must have the same type. Values retain their full precision in the browser, including 128-bit integers, and pointer-sized integers use the server target's width. Arithmetic panics on overflow, division by zero, or remainder by zero in both debug and release builds. Signed `MIN / -1`, `MIN % -1`, and negating `MIN` also panic.
- `bool`: `!`, equality comparisons, `then`, and `then_some`.
- `String` and `&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`, and comparisons.
- `Option<T>`: `is_some`, `is_none`, `unwrap`, and `expect`.
- `Result<T, E>`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, and `expect_err`.
- `Vec<T>`, `[T; N]`, and slices: `len`, `is_empty`, `get`, `index`, `first`, `last`, `to_vec`, and `to_owned`. Vectors and arrays also support `as_slice` and `clone`. Lengths and indexes are `usize`. `get` returns `None` for an out-of-bounds index; `index` panics. Both borrow the element. Elements must belong to the shared vocabulary.
- Tuples of vocabulary types.
- [`Signal`]: `get` and `set`, plus a shorter spelling for common writes: `toggle` on a `bool` signal, `increment` and `decrement` on a numeric signal, and `push_str` on a `String` signal.

Operations follow Rust semantics in both languages. For strings, `len` counts UTF-8 bytes, comparisons use code point order, and trimming uses Unicode `White_Space`. This means trimming keeps U+FEFF and removes U+0085.

# Supported syntax

Expressions use a subset of Rust's syntax:

- String, integer, `f64`, and `bool` literals.
- The unary and binary operators listed above.
- Method calls, field access, and indexing.
- Blocks, with `let` bindings of plain identifiers; the trailing expression is the block's value.
- `if`/`else` as an expression.
- Closures, optionally `async`, and `.await`.
- `loop`, `while`, `break`, `continue`, and `return`.

Unsupported syntax produces a compile error at the expression.

# Embedding JavaScript

Use `raw!` for JavaScript operations outside the supported vocabulary. Its first argument is JavaScript source. The optional second argument is equivalent Rust for server evaluation:

```rust
# use topcoat::{Result, context::Cx, runtime::signal, view::*};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
let name = signal(cx, String::new);

Ok(view! {
    $({
        let n = name.get();
        raw!("${n}.toUpperCase()", n.to_uppercase())
    })
})
# }
```

`${ident}` in the JavaScript string inserts a binding from the expression's scope. Omitting the Rust argument restricts the expression to browser execution. You are responsible for keeping the Rust and JavaScript behavior equivalent.

Signal reads in the Rust fallback determine whether the expression needs a browser binding. The fallback must therefore read the signals its JavaScript depends on, even if their initial values happen to produce a constant result.

[`Expr`]: struct.Expr.html
[`Signal`]: struct.Signal.html
[`expr!`]: macro.expr.html
