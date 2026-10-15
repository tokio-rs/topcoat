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
- `String` and `&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`, and comparisons. The result of `len` has type `usize`.
- `Option<T>`: `is_some`, `is_none`, `unwrap`, and `expect`.
- `Result<T, E>`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, and `expect_err`.
- `Vec<T>`, `[T; N]`, and slices: `len`, `is_empty`, `get`, `index`, `first`, `last`, `iter`, `to_vec`, and `to_owned`. Vectors and arrays also support `as_slice` and `clone`. Lengths and indexes are `usize`. `get` returns `None` for an out-of-bounds index; `index` panics. Both borrow the element. `iter().position(predicate)` finds a borrowed element's position. Elements must belong to the shared vocabulary.
- Tuples of vocabulary types: literals such as `(count.get(), "items")`, field access such as `pair.0`, and `clone`. Empty tuples `()` and single-element tuples `(value,)` are supported. A field of a borrowed tuple, such as `values.index(0).0`, borrows the element. A tuple renders its elements one after another with no separator. Tuples do not support comparisons.
- Structs with [`#[record]`](attr.record.html): create values with struct literals and access fields by name, such as `order.id`. Access through a borrowed record borrows the field. Records that implement `Clone` also support `clone`. Expressions cannot compare records or render them directly.
- [`Signal`]: `get` and `set`, plus a shorter spelling for common writes: `toggle` on a `bool` signal, `increment` and `decrement` on a numeric signal, and `push_str` on a `String` signal.

Operations follow Rust semantics in both languages. For strings, `len` counts UTF-8 bytes, comparisons use code point order, and trimming uses Unicode `White_Space`. This means trimming keeps U+FEFF and removes U+0085.

# Borrowed sequence lookup

Use `iter().position(predicate)` to locate the first matching element. The predicate borrows each element, including record fields. The lookup does not clone the sequence or its elements, and it does not require `Clone` on them.

```rust
use topcoat::runtime::{expr, record};

#[record]
struct Row { value: String }

let rows = [Row { value: "a".to_owned() }, Row { value: "b".to_owned() }];
let rows = rows.as_slice();
let value = "b";
let position = expr!(rows.iter().position(|row| row.value == value));
assert_eq!(position.into_evaluated_and_js().0, Some(1));
```

The result is an owned `Option<usize>`. An empty sequence or a missing match returns `None`. The predicate runs once per visited element, in order, and stops at the first match. Duplicate matches return the first position. Indexes use the server target's pointer width, just like other sequence indexes.

Only `position` is supported on the iterator. It consumes visited elements, including the matching element. Each call counts from zero within the remaining elements, as in Rust:

```rust
use topcoat::runtime::expr;

let rows = [10u32, 20, 20, 30];
let positions = expr!({
    let mut iter = rows.iter();
    let first = iter.position(|row| row == 20u32);
    let second = iter.position(|row| row == 20u32);
    let missing = iter.position(|row| row == 20u32);
    (first, second, missing)
});
assert_eq!(positions.into_evaluated_and_js().0, (Some(1), Some(0), None));
```

A fresh `rows.iter()` starts at zero again. A predicate panic propagates without visiting later elements. The failing element has already been consumed if the caller catches the panic outside the expression and resumes that iterator.

Predicates must return a boolean synchronously. Async predicates and other return types do not compile:

```rust,compile_fail
use topcoat::runtime::expr;
let rows = [1u32];
let position = expr!(rows.iter().position(async |_row| true));
```

```rust,compile_fail
use topcoat::runtime::expr;
let rows = [1u32];
let position = expr!(rows.iter().position(|row| row.clone()));
```

The iterator is local borrowed state. It cannot be an evaluated expression result, a captured value, a signal value, or a server argument. Return the position instead:

```rust,compile_fail
use topcoat::runtime::expr;
let rows = [1u32];
let iterator = expr!(rows.iter());
```

The cursor also does not implement the signal transport contract:

```rust,compile_fail
use topcoat::runtime::{SignalValue, Surrogated};
fn require_signal_value<T: SignalValue>(_: T) {}
let rows = [1u32];
let borrowed = rows.as_slice().into_surrogate();
require_signal_value(borrowed.iter());
```

It cannot outlive its sequence:

```rust,compile_fail
use topcoat::runtime::expr;
let rows = vec![1u32];
let position = expr!({
    let mut iter = {
        let temporary = rows.clone();
        temporary.iter()
    };
    iter.position(|row| row == 1u32)
});
```

Captures keep their usual behavior: an owned collection capture clones its elements, while a slice capture borrows on the server and sends a snapshot to the browser. To locate a value in current signal data, read inside the callback and borrow the rows with `signal.read().rows.iter().position(...)`. Lookup generates the same expression structure regardless of row count; only captured data grows.

# Supported syntax

Expressions use a subset of Rust's syntax:

- String, integer, `f64`, and `bool` literals.
- The unary and binary operators listed above.
- Method calls, field access, and indexing.
- Construction of options with `Some` and `None`, results with `Ok` and `Err`, tuples with tuple literals, and records with struct literals.
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
