This unpublished crate checks that runtime expressions behave the same in Rust and JavaScript. Each case compiles an expression with the real `expr!` macro, evaluates it in Rust, and runs the generated JavaScript in [V8](https://v8.dev) together with a bundle of the real browser runtime modules.

# Running the suite

From the workspace root:

```sh
cargo test -p topcoat-runtime-coherence
```

The first build downloads the V8 library. The JavaScript bundle is checked in, so running the Rust tests does not need Node or Yarn. After editing the browser runtime or its coherence adapter, rebuild the bundle from `crates/topcoat-runtime/browser`:

```sh
yarn install --frozen-lockfile
yarn build
yarn test
```

Commit the updated `dist/coherence.js` together with the source changes. CI rebuilds both JavaScript bundles and checks that they match the committed files.

# Adding expressions

Add a test in `tests/` and use [`coherent!`]:

```rust
use topcoat_runtime_coherence::coherent;

coherent!(1.0 + 2.0 * 3.0);

for text in ["", "hello", "\u{0085}\u{1f980}\u{feff}"] {
    coherent!(text.trim());
    coherent!(text.len());
}
```

Captured values go through the same serialization and hydration as in a real page. The numeric and string tests run fixed inputs through several expression shapes, so the suite is deterministic.

The default form compiles the expression as the body of a closure without arguments and calls it on each side. This lets the harness get the JavaScript before the Rust evaluation can panic. To test the expression compiled on its own, use the `direct` form:

```rust
use topcoat_runtime_coherence::coherent;

coherent!(direct => 1.0 + 2.0);
```

Direct cases must return normally on the Rust side. A panic while serializing captures or building the closure is a setup failure, not an outcome of the expression.

# Async expressions

Use the `async` form in an ordinary synchronous test. The harness runs the compiled async closure with Tokio on the Rust side and drains promise continuations in V8 on the JavaScript side, then compares the final values.

```rust
use topcoat_runtime_coherence::{Awaitable, coherent};

let value = Awaitable::ready(3.0).after_yield();
coherent!(async => {
    let number = value.await;
    number + 1.0
});
```

[`Awaitable`] provides deterministic futures that return a captured value or panic when awaited. [`after_yield`](Awaitable::after_yield) makes Rust return `Pending` once and wake the executor, and adds a promise continuation in JavaScript. This tests suspension without network requests or timers. The fixture is lazy on both sides and uses the runtime's real JavaScript `Future`. The test bundle only adds the code that hydrates the fixture.

Async cases compare final outcomes, not polling counts or scheduling order. A rejection with the runtime's JavaScript `Panic` counts as a panic, and any other rejection fails the case. `coherent!(async known "id" => expression)` checks an async expression against a recorded mismatch.

# Comparing outcomes

[`Observe`] converts Rust values into a tagged [`Value`]. The JavaScript adapter reads the value stored in each runtime object directly, without going through the runtime's own serialization and rendering. Values compare exactly, including float bits and negative zero, but all NaN payloads compare as one NaN value. Options, results, tuples, and unit keep their structural differences.

A Rust panic agrees with a JavaScript `Panic` from the runtime, whatever their messages say. Other exceptions, invalid JavaScript, and timeouts fail the case. A failure report includes the expression, both outcomes, and the generated JavaScript with its serialized captures.

# Known mismatches

A known difference between the two sides is recorded in `known-mismatches.toml`, with a name and the exact Rust and JavaScript outcomes. The test refers to it by name:

```rust
use topcoat_runtime_coherence::coherent;

let value = f64::NAN;
coherent!(known "captured_nan" => value);
```

These cases run every time. A changed outcome fails, and so does a case that unexpectedly agrees, with a message to remove the baseline. A baseline can also record the exact JavaScript compile error or exception the expression produces. Exceptions never count as agreeing outcomes, even when Rust panics. Errors from the observer, stalled promises, and timeouts always fail and cannot be recorded in a baseline.

# Coverage boundaries

The harness evaluates synchronous expressions, and synchronous or async closures without arguments. It does not set up signals, compare side effects or rendered output, send real procedure requests, or generate expressions. Its observer can represent tuples, but whether an expression can use them still depends on the compiler and on hydration.

Each evaluation gets a fresh V8 global context, runtime context, and microtask queue, with a five-second deadline. A promise that has not settled once no microtasks are left fails immediately. The harness provides the `TextEncoder.encode` operation the runtime uses, but no browser timers or networking.

The Rust side runs in the test process. Async cases have a five-second timeout, which can stop a pending future but cannot interrupt a poll that never returns. Synchronous cases have no deadline. Test expressions must therefore terminate. Expressions that might run forever need process isolation, which the harness does not have yet.

Both sides use the runtime's vocabulary types. Agreement shows that the server and the browser behave the same. It does not prove that each operation behaves like its counterpart in ordinary Rust.
