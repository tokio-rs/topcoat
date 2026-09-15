This unpublished crate checks that runtime expressions behave the same in Rust and JavaScript. Each case uses the production `expr!` compiler and runs its generated JavaScript in V8 with a bundle of the production runtime modules.

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

Captured values go through production serialization and hydration. The numeric and string matrices run fixed inputs against multiple expression shapes, so the suite is deterministic.

The default form compiles a zero-argument closure and invokes it on each side. This lets the harness retrieve the JavaScript before Rust evaluation can panic. To also test direct expression lowering, use:

```rust
use topcoat_runtime_coherence::coherent;

coherent!(direct => 1.0 + 2.0);
```

Direct cases must return normally on the Rust side. A panic during capture serialization or closure construction is a setup failure, not an expression outcome.

# Async expressions

Use `async =>` in an ordinary synchronous test. The harness runs the compiled async closure with Tokio on the Rust side and drains promise continuations in V8 on the JavaScript side, then compares the completed values.

```rust
use topcoat_runtime_coherence::{Awaitable, coherent};

let value = Awaitable::ready(3.0).after_yield();
coherent!(async => {
    let number = value.await;
    number + 1.0
});
```

[`Awaitable`] supplies deterministic futures that return a captured value or panic when awaited. `after_yield()` makes Rust return `Pending` once and wake the executor, and adds a promise continuation in JavaScript. This exercises suspension without network requests or timers. The fixture is lazy on both sides and uses the production JavaScript `Future` surrogate. Only its fixture hydration is added by the test bundle.

Async cases compare final outcomes, not polling counts or scheduling order. JavaScript runtime `Panic` rejections count as panics; other rejections fail execution. `coherent!(async known "id" => expression)` checks an async expression against a recorded mismatch.

# Comparing outcomes

[`Observe`] converts Rust values into a tagged representation. The JavaScript adapter reads the surrogate's stored value independently of production serialization and rendering. Values compare exactly, including float bits and negative zero. All NaN payloads compare as one NaN value. Options, results, tuples, and unit retain their structural distinctions.

A Rust panic agrees with a JavaScript runtime `Panic`, regardless of message wording. Other exceptions, invalid JavaScript, and timeouts fail the case. Failure reports include the expression, both outcomes, and generated JavaScript with serialized captures.

# Known mismatches

A known discrepancy has a named case and exact Rust and JavaScript outcomes in `known-mismatches.toml`:

```rust
use topcoat_runtime_coherence::coherent;

let value = f64::NAN;
coherent!(known "captured_nan" => value);
```

These cases execute on every run. A changed outcome fails, and an unexpected pass fails with an instruction to remove the baseline. A baseline can also record an exact JavaScript compilation error. Unexpected runtime exceptions and timeouts always fail.

# Coverage boundaries

The harness evaluates synchronous expressions and synchronous or async closures without arguments. It does not yet initialize signal fixtures, compare side effects or rendered output, drive real procedure requests, or generate expression source. Its observer can represent tuples, but expression support still depends on the production compiler and hydration code.

V8 gets a fresh global context, runtime context, and microtask queue for each evaluation, with a five-second deadline. An unsettled promise with no runnable microtasks fails immediately. The host supplies the `TextEncoder.encode` operation used by the runtime, but no browser timers or networking.

Rust evaluation runs in the test process. Async cases have a five-second cooperative timeout, which can stop a pending future but cannot interrupt a poll that never returns. Synchronous cases have no deadline. Test expressions must terminate; process isolation is needed before adding potentially unbounded Rust programs.

Both sides use the compiler's surrogate vocabulary. Agreement establishes server/client coherence; it does not independently prove agreement with every operation in ordinary Rust.
