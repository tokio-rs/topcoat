This crate checks that runtime expressions produce the same outcomes in Rust and JavaScript. It compiles each case with `expr!` and runs the generated JavaScript in V8 with the browser runtime.

# Running the suite

From the workspace root:

```sh
cargo test -p topcoat-runtime-coherence
```

The first build downloads V8. The JavaScript bundle is checked in, so Rust tests do not require Node or Yarn. After editing the browser runtime or its test adapter, rebuild the bundle from `crates/topcoat-runtime/browser`:

```sh
yarn install --frozen-lockfile
yarn build
yarn test
```

Commit changes to `dist/coherence.js` with the source changes. CI rebuilds both JavaScript bundles and checks that they match the committed files.

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

Captured values use the same serialization and hydration as application expressions. Use fixed inputs to keep cases reproducible.

The default form compiles a closure and calls it in each language. This captures the generated JavaScript before Rust evaluation can panic. To test an expression without a closure, use:

```rust
use topcoat_runtime_coherence::coherent;

coherent!(direct => 1.0 + 2.0);
```

Direct cases must return normally on the Rust side. A panic during capture serialization or closure construction is a setup failure, not an expression outcome.

# Async expressions

Use `async =>` in a synchronous test. The harness runs an async closure with Tokio and its JavaScript equivalent in V8, then compares their completed values.

```rust
use topcoat_runtime_coherence::{Awaitable, coherent};

let value = Awaitable::ready(3.0).after_yield();
coherent!(async => {
    let number = value.await;
    number + 1.0
});
```

[`Awaitable`] supplies reproducible futures that return a captured value or panic when awaited. `after_yield()` suspends once in Rust and adds a promise continuation in JavaScript. Use it to test suspension without networking or timers. Both sides defer execution until awaited.

Async cases compare final outcomes, not polling counts or scheduling order. JavaScript runtime `Panic` rejections count as panics; other rejections fail execution. `coherent!(async known "id" => expression)` checks an async expression against a recorded mismatch.

# Comparing outcomes

[`Observe`] converts Rust values to a comparison format. The JavaScript adapter reads runtime values independently of their serialization and rendering. Comparisons preserve type structure and exact values, including float bits and negative zero. All NaN payloads count as the same value.

A Rust panic agrees with a JavaScript runtime `Panic`, regardless of message wording. Other exceptions, invalid JavaScript, and timeouts fail the case. Failure reports include the expression, both outcomes, and generated JavaScript with serialized captures.

# Known mismatches

A known discrepancy has a named case and exact Rust and JavaScript outcomes in `known-mismatches.toml`:

```rust
use topcoat_runtime_coherence::coherent;

let value = f64::NAN;
coherent!(known "captured_nan" => value);
```

Known mismatches run with the other cases. A changed outcome fails. If the two languages agree, the test fails and asks you to remove the baseline.

A baseline can record an exact JavaScript compilation error or evaluation exception. Such an exception does not count as agreement with a Rust panic. Observer errors, stalled promises, and timeouts always fail, even with a baseline.

# Coverage boundaries

The harness compares the final outcomes of supplied expressions and closures without arguments. It does not compare browser interactions, rendered output, or side effects. Cases remain subject to the expression compiler's supported syntax and types.

Each V8 evaluation gets fresh global and runtime contexts and a microtask queue, with a five-second deadline. A pending promise with no runnable microtasks fails immediately. Browser timers and networking are unavailable.

Rust evaluation runs in the test process. Async cases have a five-second cooperative timeout, which can stop a pending future but cannot interrupt a poll that never returns. Synchronous cases have no deadline. Test expressions must terminate; process isolation is needed before adding potentially unbounded Rust programs.

Both sides use runtime surrogate types. Agreement checks consistency between the server and browser, but does not independently verify every operation against ordinary Rust.
