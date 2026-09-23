The [`expr!`] macro creates an expression that runs on the server and in the browser. In a `view!` body, write it as `$(...)`:

Expressions support a subset of Rust. The supported types, methods, and syntax are listed below.

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

The server evaluates the expression for the initial HTML. The browser updates its output whenever a signal it reads changes, without a server request.

The macro returns an [`Expr`] containing the server result and the JavaScript expression.

Expressions run synchronously during rendering. An expression that reads no signals renders as a fixed value. Event-handler closures run only when their event fires in the browser.

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

Captured values must use the supported types below. They are cloned, so the surrounding Rust code can keep using them. Ordinary values become snapshots in the browser and do not receive later server changes. Capturing an owned collection clones its elements. Capturing a slice borrows its elements on the server but still sends a snapshot to the browser.

A captured `Expr<T>` behaves as its result type `T`. The server uses its existing result, while the browser evaluates it at each use. Its signal reads remain reactive, so expressions can build on each other:

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

Runtime expressions support the following types and operations:

- `f64`: arithmetic (`+`, `-`, `*`, `/`), comparisons, and negation. Floating-point literals are `f64`. Rendered text follows Rust's `Display`, so it is always positional, however large or small: `inf`, `-inf`, and `-0` are spelled the Rust way rather than the JavaScript way.
- Rust integer types: arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons, and negation for signed types. Unsuffixed integer literals are `usize`; use a suffix for another type, such as `42u64` or `-1i32`. Operands must have the same type. Values retain their full precision in the browser, including 128-bit integers, and pointer-sized integers use the server target's width. Arithmetic panics on overflow, division by zero, or remainder by zero in both debug and release builds. Signed `MIN / -1`, `MIN % -1`, and negating `MIN` also panic.
- `bool`: `!`, equality comparisons, `then`, and `then_some`.
- `String` and `&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`, and comparisons.
- `Option<T>`: `is_some`, `is_none`, `unwrap`, and `expect`.
- `Result<T, E>`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, and `expect_err`.
- `Vec<T>`, `[T; N]`, and slices: `len`, `is_empty`, `get`, `index`, `first`, `last`, `to_vec`, and `to_owned`. Vectors and arrays also support `as_slice` and `clone`. Lengths and indexes are `usize`. `get` returns `None` for an out-of-bounds index; `index` panics. Both borrow the element. Elements must belong to the shared vocabulary.
- Tuples of vocabulary types.
- [`Signal`]: `get` and `set`, plus a shorter spelling for common writes: `toggle` on a `bool` signal, `increment` and `decrement` on a numeric signal, and `push_str` on a `String` signal.

Operations follow Rust's behavior in both environments. For example, string `len` counts UTF-8 bytes, string comparisons use code point order, and `trim` uses the Unicode `White_Space` set.

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

Use `raw!` to write JavaScript directly. Its first argument is JavaScript source. Its optional second argument is the equivalent Rust expression to evaluate on the server:

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

`${ident}` inserts a binding from the expression's scope. Omit the Rust argument only in browser-only code, such as an event-handler body. You are responsible for making the Rust and JavaScript behave the same way.

The Rust fallback must read every signal the JavaScript depends on, even if the initial result is constant. These reads tell the browser when to update the expression.

[`Expr`]: struct.Expr.html
[`Signal`]: struct.Signal.html
[`expr!`]: macro.expr.html
