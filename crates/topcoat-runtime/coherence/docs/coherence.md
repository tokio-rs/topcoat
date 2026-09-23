This crate tests whether runtime expressions produce the same outcomes in Rust and JavaScript. Tests use the production `expr!` compiler and browser runtime.

# Running the suite

From the workspace root:

```sh
cargo test -p topcoat-runtime-coherence
```

The first build downloads the V8 library. The coherence bundle is checked in, so running the Rust tests does not require Node or Yarn. After editing the browser runtime or its coherence adapter, rebuild it from `crates/topcoat-runtime/browser`:

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

Captured values use the same serialization and hydration as application expressions.

The default form runs the expression inside a closure so panics can be compared. To test an expression without that wrapper, use:

```rust
use topcoat_runtime_coherence::coherent;

coherent!(direct => 1.0 + 2.0);
```

Direct cases must return normally on the Rust side. A panic during capture serialization or closure construction is a setup failure, not an expression outcome.

# Async expressions

Use `async =>` in a synchronous test to compare the completed results of an async expression:

```rust
use topcoat_runtime_coherence::{Awaitable, coherent};

let value = Awaitable::ready(3.0).after_yield();
coherent!(async => {
    let number = value.await;
    number + 1.0
});
```

[`Awaitable`] provides futures that return a value or panic when awaited. Call `after_yield()` to test suspension before completion without relying on timers or network requests.

Async cases compare final outcomes, not polling counts or scheduling order. JavaScript runtime `Panic` rejections count as panics; other rejections fail execution. `coherent!(async known "id" => expression)` checks an async expression against a recorded mismatch.

# Comparing outcomes

Values compare exactly, including their structure, float bits, and negative zero. All NaN payloads compare as one NaN value. The comparison uses [`Observe`] and a JavaScript adapter independently of production serialization and rendering.

A Rust panic agrees with a JavaScript runtime `Panic`, regardless of message wording. Other exceptions, invalid JavaScript, and timeouts fail the case. Failure reports include the expression, both outcomes, and generated JavaScript with serialized captures.

# Known mismatches

A known discrepancy has a named case and exact Rust and JavaScript outcomes in `known-mismatches.toml`:

```rust
use topcoat_runtime_coherence::coherent;

let value = f64::NAN;
coherent!(known "captured_nan" => value);
```

These cases execute on every run. A changed outcome fails, and an unexpected pass fails with an instruction to remove the baseline. A baseline can also record an exact JavaScript compilation error or exception from evaluating the expression. Exceptions never count as coherent outcomes, even when Rust panics. Errors from the observer, stalled promises, and timeouts always fail and cannot be accepted by a baseline.

# Scope and limits

The harness compares expression results and panics. Use browser or integration tests for interactions and rendered output. Supported expressions depend on the production compiler and runtime.

JavaScript runs in an isolated V8 context with a five-second deadline. An unsettled promise with no runnable microtasks fails immediately. Browser timers and networking are unavailable.

Rust evaluation runs in the test process. Async cases have a five-second cooperative timeout, which cannot interrupt a poll that never returns. Synchronous cases have no deadline. Every test expression must terminate.

Both sides use the compiler's surrogate types. Agreement checks server/browser consistency, not agreement with ordinary Rust outside the expression language.
