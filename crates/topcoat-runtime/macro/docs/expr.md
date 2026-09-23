The [`expr!`] macro compiles one Rust expression twice: into ordinary Rust that the server runs during the initial render, and into equivalent JavaScript that ships with the page and runs again in the browser. Inside a `view!` body, a runtime expression is written `$(...)`, which expands to [`expr!`]. You rarely need to call the macro directly.

Every expression must behave the same in both languages, so only a subset of Rust is supported: a small vocabulary of types and methods, and a limited set of syntax. Both are listed below.

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

The server evaluates the expression once to produce the initial HTML. In the browser, the compiled JavaScript runs again whenever a signal it reads changes, and updates the DOM in place. There is no wasm bundle, no client build step, and no server round-trip.

The macro expands to an [`Expr`] value, which holds both the server's result and the JavaScript source.

The server evaluates the expression synchronously and records whether it reads any signals. An expression that reads no signals, such as a literal or a captured ordinary value, renders as plain HTML without any browser binding. Building an event handler closure does not run its body, but the closure's JavaScript is still sent so the handler can run in the browser.

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

The expression captures a clone of the value, so the surrounding code can keep using it. The clone is serialized into the page during the render and becomes a constant in the JavaScript. It is a snapshot: the browser keeps the value from the render, and later changes on the server do not reach it. A captured value must belong to the shared vocabulary described below. Cloning an owned collection clones its elements. Capturing a slice borrows the Rust elements, but the browser still receives a snapshot.

A captured [`Expr<T>`][`Expr`] behaves like a value of type `T` inside the expression. Its JavaScript is inlined at each use, so its signal reads stay reactive, also inside an event handler. The server reuses the value it already computed, and the enclosing expression reads signals if the captured one did. Capturing an expression that reads no signals keeps the enclosing expression free of browser bindings as well.

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

Expressions work with a fixed set of types that exist on both sides. Each type exposes a subset of its Rust API. The most useful members are:

- `f64`: arithmetic (`+`, `-`, `*`, `/`), comparisons, and negation. Floating-point literals are `f64`. Rendered text follows Rust's `Display`, so it never uses exponential notation, and `inf`, `-inf`, and `-0` are spelled the Rust way rather than the JavaScript way.
- Rust integer types: arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons, and negation for signed types. Integer literals without a suffix are `usize`. Use a suffix for another type, such as `42u64` or `-1i32`. Both operands must have the same type. Values keep their full precision in the browser, including 128-bit integers, and `usize` and `isize` use the server target's width. Arithmetic panics on overflow, division by zero, or remainder by zero, in both debug and release builds. Signed `MIN / -1`, `MIN % -1`, and negating `MIN` also panic.
- `bool`: `!`, equality comparisons, `then`, and `then_some`.
- `String` and `&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`, and comparisons. Note that `len` returns an `f64`.
- `Option<T>`: `is_some`, `is_none`, `unwrap`, and `expect`.
- `Result<T, E>`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, and `expect_err`.
- `Vec<T>`, `[T; N]`, and slices: `len`, `is_empty`, `get`, `index`, `first`, `last`, `to_vec`, and `to_owned`. Vectors and arrays also support `as_slice` and `clone`. Lengths and indexes are `usize`. `get` returns `None` for an index out of bounds, while indexing panics. Both borrow the element. Elements must belong to the shared vocabulary.
- Tuples of vocabulary types, whose fields are read with `.0`, `.1`, and so on.
- [`Signal`]: `get`, `read`, and `set`, plus shorter forms for common writes: `toggle` on a `bool` signal, `increment` and `decrement` on a numeric signal, and `push_str` on a `String` signal. Writes only work in the browser, so they belong inside event handler closures.

Each member has to mean the same thing on both sides. Rust's behavior is the reference, and the JavaScript matches it even where that differs from normal JavaScript behavior. For example, `len` counts UTF-8 bytes rather than UTF-16 code units, string comparisons order by code point rather than by UTF-16 code unit, and `trim` and its siblings strip the Unicode `White_Space` set rather than the ECMAScript one, so U+FEFF is kept and U+0085 is removed.

# Supported syntax

Expressions support this subset of Rust syntax:

- String, integer, `f64`, and `bool` literals.
- The unary and binary operators listed above.
- Method calls, field access, and indexing.
- Calls to [procedures](attr.procedure.html).
- Blocks with `let` bindings of plain identifiers. The last expression is the block's value.
- `if` and `else` as an expression.
- Closures, optionally `async`, and `.await`.
- `loop`, `while`, `break`, `continue`, and `return`, without labels.

Anything else, such as `match`, struct expressions, or paths with more than one segment, is rejected with a compile error that points at the unsupported code.

# Embedding JavaScript

The `raw!` macro inserts hand-written JavaScript for parts of an expression the vocabulary does not cover. Its first argument is the JavaScript source. The optional second argument is the equivalent Rust, which the server runs instead:

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

Inside the JavaScript string, `${ident}` inserts a binding from the expression's scope. In the Rust argument, local bindings and captured variables are ordinary Rust values, so their full API is available. Without the Rust argument, the server panics when it evaluates the expression, so that form only works where the expression runs purely in the browser, such as inside an event handler. In both forms it is up to you to keep the two sides equivalent.

The server decides whether the expression needs a browser binding by watching the signal reads of the Rust argument. The Rust argument must therefore read every signal its JavaScript depends on, even when their initial values produce a constant result.

[`Expr`]: struct.Expr.html
[`Signal`]: struct.Signal.html
[`expr!`]: macro.expr.html
