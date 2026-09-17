The experimental compiler driver builds client Wasm bundles from ordinary Rust `$(...)` expressions in Topcoat pages and shards. Templates and signal storage stay in JavaScript. Rust evaluates expressions and reads or writes signals through host imports.

## Run the Topcoat example

Install the analyzer toolchain, the client target, wasm-bindgen CLI 0.2.128, esbuild, and Binaryen (for wasm-opt):

```sh
rustup toolchain install nightly-2026-08-05 --component rustc-dev,rust-src,llvm-tools-preview
rustup target add wasm32-unknown-unknown --toolchain stable
cargo install wasm-bindgen-cli --version 0.2.128 --locked
```

Install the browser dependencies with `yarn install --frozen-lockfile` in `crates/topcoat-runtime/browser`. From the repository root, run:

```sh
node tools/topcoat-split/examples/topcoat.mjs --release
python3 -m http.server 8082 --bind 127.0.0.1 --directory target/topcoat-native/public
```

Open `http://localhost:8082`. `ESBUILD` selects an esbuild executable. Release builds require wasm-opt on PATH, or a `WASM_OPT` environment variable pointing to it. `TOPCOAT_SPLIT_CLIENT_TOOLCHAIN` selects the client compiler, defaulting to stable. `TOPCOAT_SPLIT_ANALYZER_TOOLCHAIN` can select an installed alias for the pinned analyzer revision. The analyzer's build script checks the exact compiler commit.

The script defaults to `--release`. Release uses optimized Rust code, strips compiler metadata, shortens Wasm import/export names, and minifies JavaScript. The build updates Wasm and its generated JavaScript bindings together. The JavaScript `dispatch` API keeps its name. Generated bindings are internal build artifacts and do not include TypeScript declarations.

For readable Wasm names, Rust debug information, and unminified JavaScript with source maps, use:

```sh
node tools/topcoat-split/examples/topcoat.mjs --debug
python3 -m http.server 8083 --bind 127.0.0.1 --directory target/topcoat-native-debug/public
```

Debug builds do not need wasm-opt. Both profiles use the same typed signal bridge, while debug retains compiler diagnostics and support code. Client artifacts, manifests, HTML, and size reports are kept in separate output directories. The server analysis and rendering steps use a development build in both cases.

The example in `examples/runtime/src/bin/wasm-native.rs` uses the real `view!`, `#[page]`, `#[component]`, and `#[shard]`. Each component has separate Text, numeric, and boolean signals. Expressions call a private numeric helper, update quantities, concatenate Unicode text, compare cloned text, and toggle an attribute. The page renders two independent instances of a shared component; a shard renders another. A Product struct, vector processing, and async initialization stay on the server and are excluded from the client crates.

The script writes generated Rust, manifests, rendered HTML, JavaScript, and Wasm under `target/topcoat-native`. It hydrates actual server HTML in Happy DOM and verifies independent signal updates. It also checks primitive bounds, Unicode validity, error cleanup, and nested calls. Generated client crates must have no serde dependency. Release glue must have no TextEncoder, TextDecoder, or JSON calls, and release modules must not export byte allocation functions. Allocation exports are checked before their names are shortened. `sizes.json` reports raw, gzip, and Brotli sizes for each module. Inspect `public/app.js` and `public/index.html` separately when comparing total downloads.

## Build stages

The orchestration script builds the server twice with `--cfg topcoat_wasm`. The analysis build uses the pinned compiler and sets `RUSTC_WORKSPACE_WRAPPER` to the driver. The macros emit hidden expression and owner markers. rustc supplies resolved paths, inferred types, method definitions, and closure captures. This mode bypasses the JavaScript expression transpiler.

The driver follows same-crate component references from pages and shards. Each owner gets a standalone client crate with its expressions and reachable component expressions. A shard starts a separate bundle. Components used by several owners are included in each bundle. Expressions have one dispatcher entry per source location; component instances supply separate capture environments.

The driver retains selected Rust source, qualifies resolved paths, and includes the types, inherent methods, and helper functions needed by each bundle. It replaces Topcoat signal operations with a small host adapter. Generated crates depend on wasm-bindgen, without serde or the Topcoat server runtime. The developer's client compiler checks and compiles the generated source for Wasm.

The final server build reads the generated manifest to bind captured variables in compiler-reported order. Its HTML references expression IDs and serialized captures. The loader initializes the Wasm modules and installs their dispatch bridge before starting Topcoat hydration. Server HTML, manifests, and client modules must come from the same build.

Normal builds without `topcoat_wasm` keep the JavaScript expression backend. The orchestration lives in `tools/topcoat-split/examples/topcoat.mjs`.

## Data crossing the boundary

JavaScript owns each signal's current value. Server state arrives in an opaque `Wasm` envelope. Captured signals pass their existing IDs. Other supported captures pass snapshots. This server transfer can use serde; it is separate from the browser-to-Wasm bridge.

On `signal.get()`, the host reads the JavaScript signal and returns its primitive value or string reference. That read participates in the current reactive effect. On `signal.set(value)`, the host checks the value and updates the same signal. Numeric captures and signal reads return f64 directly; boolean captures and reads use a boolean import. Writes use matching typed imports. JavaScript validates values before ABI coercion. Text and unit use JS value handles, and expression results currently use a common JS value return. There is no JSON encoding or parsing on this path.

`topcoat::client::Text` has native storage on the server and a JavaScript string reference in Wasm. Cloning, equality, concatenation, and emptiness checks do not transfer string bytes into Wasm memory. Text has value semantics and contains Unicode scalar values; the host rejects lone UTF-16 surrogates without transcoding. There is no implicit `&str` conversion. Construct Text on the server and capture it or put it in a signal. Client-side construction from string literals is not supported yet.

Expression results use adapters for unit, bool, f64, fixed-width integers up to 32 bits, and Text. DOM bindings receive ordinary Topcoat browser values. Invocation frames are released on return or failure, and handlers retain their capture environment until called. The typed bridge does not impose `no_std` on application code; generated bundles are checked for the specific allocation and conversion costs this example must avoid.

## Supported subset

This is an integrated experimental backend, not an arbitrary Rust program slicer. Its current boundaries are:

- Native signal operations are `get` and `set`. Event handlers must be synchronous with an unused event argument. Event APIs, procedures, and the existing signal convenience methods are not mapped yet.
- Signal values implement the sealed `ClientValue` trait. Text, bool, unit, f64, and fixed-width integers up to 32 bits are supported. Structs, collections, ordinary String values, and borrowed strings cannot cross this bridge. There is no automatic JSON fallback. Manual trait implementations and destructors on extracted local types remain unsupported.
- Projected and mutable captures are rejected. Supported by-value and shared-reference captures become snapshots; signal handles preserve identity. Generic expressions and several generic type forms are not supported.
- Wider and pointer-sized integers cannot cross the boundary. f32 is also excluded until there is a matching browser display adapter. Signal reads and writes validate types and integer bounds; f64 state must be finite.
- Extraction covers local items and supported standard-library operations. It does not extract arbitrary external crates or automatically configure their client features. Same-crate owner discovery requires statically visible component references; indirect rendering and expressions without a discoverable page or shard owner are unsupported.
- The server must type-check on the pinned analyzer compiler. Server-only `cfg` choices and macro expansion cannot generally be reproduced for a different target by copying source. Unsupported source forms are rejected where detected; the client compiler checks the generated crate.

Rust iteration inside an expression works when its dependencies are supported. This does not add a client-side list template renderer. Separate Wasm bundles each have their own memory and runtime code; they do not dynamically link shared Rust objects or code. The browser loader currently loads both example bundles eagerly and reuses the first installed entry for a shared expression.

These restrictions mean the existing examples are not all Wasm-compatible. The workspace tests exercise the normal backend for regressions, while the native example exercises the integrated backend.

## Analyzer fixtures

The extraction-only fixtures use an explicit `__topcoat_split_root` marker and a small ordinary Rust signal container. They test source selection independently of the browser bridge:

```sh
cargo +nightly-2026-08-05 run --manifest-path tools/topcoat-split/Cargo.toml -- \
    --out-dir target/driver-split -- \
    tools/topcoat-split/tests/fixtures/product.rs --edition=2024
node tools/topcoat-split/examples/product.mjs
```

Their size measurements do not represent the typed Topcoat signal bridge. All bundles are analyzed and their dispatchers validated before files are written. A failed run leaves files from earlier successful runs in place, so callers must check the exit status.

## Verification

```sh
cargo +nightly-2026-08-05 test --manifest-path tools/topcoat-split/Cargo.toml
cargo +nightly-2026-08-05 clippy --manifest-path tools/topcoat-split/Cargo.toml \
    --all-targets -- -D warnings
node tools/topcoat-split/examples/topcoat.mjs --release
node tools/topcoat-split/examples/topcoat.mjs --debug
node tools/topcoat-split/examples/bridge-checks.mjs
```

Analyzer tests inspect bundle membership, compile and execute generated Rust with the client toolchain, and compile fixtures for Wasm. They check deterministic output and reject unsupported captures, attributes, destructors, and unsupported boundary types. Native tests cover Text semantics and server serialization. The host checks cover all supported primitive bounds and invalid writes; the integration script exercises actual Topcoat HTML and Wasm together.

The bridge checks compile and execute a fixture through both debug and release pipelines. They exercise captures, signal reads, and writes for every supported type, including integer limits, negative zero, invalid values, and Unicode. They also check that release bindings use shortened names without compiler metadata, while debug retains readable exports and the Wasm name section.
