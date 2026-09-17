# Rust expressions in Wasm: spike report

This spike compiles supported `$(...)` expressions to Wasm. The app keeps its `view!`, signals, event handlers, and Rust `async`/`.await` syntax.

The main size rule: put Rust computation in Wasm. Keep templates, signal storage, strings, DOM updates, and HTTP transport outside it.

The code examples are source fragments or simplified generated code. The implementation links at the end show the complete paths.

## Result

Coffee-shop release builds, measured on September 17, 2026:

| Bundle | Expressions | Raw | gzip | Brotli |
| --- | ---: | ---: | ---: | ---: |
| Menu | 5 | 262 B | 192 B | 183 B |
| Drink | 7 | 2,302 B | 1,422 B | 1,243 B |
| Shared allocator | - | 1,836 B | 950 B | 917 B |
| Total | 12 | 4,400 B | 2,564 B | 2,343 B |

Drink plus allocator costs **4,138 bytes raw**. Later allocating bundles can reuse the allocator download.

These are Wasm file sizes. They exclude JavaScript, HTML, CSS, and server state. Compressed totals add the sizes of separately compressed files. The example loader currently loads both page bundles eagerly.

Measured with Rust 1.98.1, wasm-bindgen 0.2.128, Binaryen 132, and wee_alloc 0.4.5. This is a working experimental backend. It is not yet a general Rust code splitter or a completed framework benchmark comparison.

## 1. What the app writes

Inside a page:

```rust
let name = Text::from(drink.name.clone());
let price = drink.price;
let quantity = signal(cx, || 1.0);
let confirmation = signal(cx, Text::new);

view! {
    <p>$(quantity.get() * price)</p>

    <button @click=$(|_e: Event| quantity.increment())>
        "+"
    </button>

    <button @click=$(async move |_e: Event| {
        let message = place_order(name.clone(), quantity.get()).await;
        confirmation.set(message);
    })>
        "Order"
    </button>

    <p :hidden=$(confirmation.get().is_empty())>
        $(confirmation.get())
    </p>
}
```

`drink` and the database query stay on the server. The client needs only `name`, `price`, and two signal handles.

| Server | Browser JavaScript | Wasm |
| --- | --- | --- |
| Database queries | Signal values and dependency tracking | Arithmetic and branches |
| Initial HTML and state | Template bindings and DOM updates | Extracted expression bodies |
| Shard rendering | Strings and event snapshots | Selected Rust helpers |
| Procedure implementations | HTTP, JSON, and Promise scheduling | Rust futures and their local state |

## 2. Use rustc to discover the client code

We use a small tool built with `rustc_driver`. It runs after server type checking.

The macros mark each expression as a closure. Simplified expansion:

```rust
// Input
$(quantity.get() * price)

// Hidden marker in the server analysis build
__topcoat_wasm_root(metadata, || quantity.get() * price)
```

rustc supplies facts that a text scan cannot reliably infer:

```text
expression
  captures:
    quantity: &Signal<f64>
    price:    &f64
  result: f64
  calls: the resolved Signal::get method
  owner: the containing page or component
```

The driver reads typed HIR and type-checking results, including `tcx.closure_captures(...)`. It uses resolved item identities to follow functions, types, and inherent methods.

For example:

```rust
fn total(quantity: f64, price: f64) -> f64 {
    quantity * price
}

// The driver finds and includes total().
$(total(quantity.get(), price))
```

The output is a new Rust crate. The driver copies supported source and qualifies resolved paths. It maps Topcoat signal and Text operations to a small client adapter.

```text
Server Rust
    |
    | pinned rustc_driver: resolve types, captures, and dependencies
    v
Generated client Rust
    |
    | client rustc: type-check and compile for wasm32-unknown-unknown
    v
Wasm + generated JS bindings
```

We do not compile dumped MIR into Wasm. Earlier MIR exploration led to this compiler-driven extraction approach. The client compiler compiles ordinary generated Rust.

The analyzer uses pinned `nightly-2026-08-05` because rustc's internal APIs are version-specific. The client compiler is separate and defaults to stable. The server must also type-check on the analyzer's compiler.

## 3. Pages and shards define bundles

`#[page]` and `#[shard]` identify bundle owners. They do not become whole-page Wasm entry points.

```text
page A
  expression 0
  component Counter -> expressions 1, 2
  shard Summary     -> separate bundle

page B
  component Counter -> expressions 0, 1 in B's bundle
```

The driver follows visible component references in the same crate. Shared component expressions can appear in several bundles. There is no general shared-Rust-code linker.

Each bundle has one dispatcher. Simplified generated code:

```rust
fn expr_0(quantity: &Signal<f64>, price: &f64) -> f64 {
    quantity.get() * price
}

#[wasm_bindgen]
pub fn dispatch(index: u32) -> JsValue {
    match index {
        0 => {
            let quantity = Signal::<f64>::new(0); // capture slot 0
            let price = f64::capture(1);          // capture slot 1
            expr_0(&quantity, &price).into_js()
        }
        // Other expressions in this page or shard.
        _ => core::arch::wasm32::unreachable(),
    }
}
```

One source expression gets one entry. Rendering the same component twice creates two capture environments, not two copies of its code within that bundle.

## 4. Build the server twice

```text
1. Analysis server build
   --cfg topcoat_wasm
   RUSTC_WORKSPACE_WRAPPER=topcoat-split
   -> generated client crates + capture/bundle manifest

2. Client build
   generated crates -> Wasm -> wasm-bindgen -> size passes

3. Final server build
   --cfg topcoat_wasm
   TOPCOAT_WASM_MANIFEST=...
   -> HTML that binds expressions using the manifest

4. Browser startup
   load Wasm -> install expression bridge -> hydrate HTML
```

`topcoat_wasm` selects the native expression expansion. It bypasses the JS expression transpiler. Normal builds keep the existing JS backend.

The final server build uses the compiler-reported capture order. Its generated JS binding has this shape:

```javascript
globalThis.__topcoatWasm(cx, expressionId, [quantitySignal, priceSnapshot])
```

This is dispatch glue. The expression's multiplication, branching, and Rust calls execute in Wasm.

HTML, manifests, Wasm, and bindings must come from the same build. Expression IDs include source location and a source hash. Stale analysis cannot safely bind a changed expression.

## 5. Signals are host handles

The client `Signal<T>` stores a capture slot. JavaScript owns the actual signal.

```text
Rust: quantity.get()
  -> read_number(slot)
  -> JS reads the current frame's signal
  -> JS tracks the reactive dependency
  -> Wasm receives an f64

Rust: quantity.set(next)
  -> write_number(slot, next)
  -> JS validates the value
  -> JS updates the signal
  -> existing bindings update the DOM
```

Conceptual host code; the implementation also handles Topcoat's value wrappers:

```javascript
function read_number(slot) {
    return checked(frame().captures[slot].get());
}

function write_number(slot, value) {
    frame().captures[slot].set(checked(value));
}
```

Each invocation pushes a frame containing its captures. Nested calls get another frame. Async handlers restore their frame on every poll.

Capture rules:

| Capture | Client meaning |
| --- | --- |
| `Signal<T>` | Handle to the existing JS signal |
| Supported value, including a shared reference | Snapshot of the value |
| Mutable reference or projected capture | Rejected |
| Arbitrary struct, collection, or closure | Unsupported at this boundary |

Supported values are `Text`, `bool`, `()`, `f64`, and fixed-width integers up to 32 bits. JavaScript validates types and numeric bounds before ABI coercion. Supported integers fit exactly in `f64`; floating-point state must be finite.

Local Rust types can be dependencies of extracted computation where supported. That does not make those types valid signal payloads. A generalized user-defined `Closable` protocol is not implemented.

## 6. Text keeps string bytes in JavaScript

`Text` has two representations:

```rust
// Server: native text storage, with serialization and as_str().
let name = Text::from(drink.name.clone());

// Generated client representation, simplified:
struct Text(JsValue); // handle to a JS string
```

```rust
$(name.clone())
$(name.concat(&suffix))
$(confirmation.get().is_empty())
```

These operations use JS references and host calls. They do not copy string bytes into Wasm, decode UTF-8, or allocate Rust Strings. Concatenation can allocate a new string in JavaScript.

Create literals on the server and capture them:

```rust
let suffix = Text::from("!");

view! {
    <p>$(name.concat(&suffix))</p>
}
```

Client `Text::from("!")`, `&str` access, byte indexing, and implicit String conversion are outside this spike's supported client API. Text preserves Unicode scalar values; the bridge rejects lone UTF-16 surrogates.

There are two distinct data paths:

```text
Server <-> browser: serialization remains, including procedure HTTP JSON.
Browser JS <-> Wasm: primitives and handles; no generic serde/JSON bridge.
```

The generated Wasm crate has no serde dependency. It also does not contain the template text or DOM runtime.

## 7. Keep native Rust async; use a small scheduler bridge

The order handler uses an ordinary Rust future:

```rust
$(async move |_e: Event| {
    let message = place_order(name.clone(), quantity.get()).await;
    confirmation.set(message);
})
```

The server keeps the procedure implementation. The client gets a stub with the same client arguments and result type. Simplified stub:

```rust
fn place_order(name: Text, quantity: f64) -> impl Future<Output = Text> {
    let args = Arguments::new(); // a JS array handle
    args.push(&name.into_js());
    args.push(&quantity.into_js());
    procedure(0, args)
}
```

The stub returns its future directly. The runtime uses `poll_fn` directly too. This avoids extra forwarding async state machines. The request starts on the first poll, not when the future is created.

```text
JS event callback
  -> dispatch creates a Rust task
  -> JS polls immediately, preserving prevent_default() timing
  -> procedure import starts the JS request
  -> Rust returns Pending
  -> JS Promise settles
  -> JS restores captures and polls the same Rust task
  -> Rust continues after .await
  -> JS frees the task on completion or rejection
```

Task storage:

```rust
#[wasm_bindgen]
pub struct Task {
    future: Pin<Box<dyn Future<Output = JsValue>>>,
}

#[wasm_bindgen]
pub fn resume(task: &mut Task) -> bool {
    task.future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_ready()
}
```

wasm-bindgen guards the mutable borrow. There is no extra `RefCell<Option<...>>` inside Task. Completed futures stay in their task until JS releases it.

```javascript
// Private generated bridge API; uses the existing Rust destructor.
export function cancel(task) {
    _assertClass(task, Task);
    task.free();
}
```

This removes a separate Rust cancellation export. Finalization and explicit cleanup share a destructor path.

Supported: sequential procedure awaits and concurrent handlers. Each handler can have one outstanding procedure. Arbitrary futures that need their own wakeups are unsupported. A no-op waker is sufficient only for this restricted scheduler.

Future storage costs runtime memory. The module also needs code to poll, drop, allocate, and guard that storage. Those instructions explain much of the async bundle's file size. Reserved stack/heap capacity is not itself downloaded as that many bytes.

## 8. Compile a small runtime

Generated crates start with:

```rust
#![no_std]
extern crate alloc;

#[global_allocator]
static ALLOCATOR: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[panic_handler]
fn panic_abort(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
```

They use wasm-bindgen and wee_alloc. They exclude the Topcoat server runtime, serde, js-sys, and wasm-bindgen-futures. Rust `core`, `alloc`, and supported extracted helpers remain available.

Release settings:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

The build temporarily preserves names for its link passes. It then runs `wasm-opt -O3 -Oz`, removes debug/producer metadata, and shortens import/export names. It updates the JS bindings to match.

Release binding generation also preserves the JS handle representation. Enabling wasm-bindgen's reference-table path in this experiment retained extra Wasm allocation machinery.

Rust panic conditions trap in both profiles. Release also replaces known panic/allocation-failure/binding-error functions with traps before optimization. Debug keeps descriptive binding errors and readable names.

A trap does not unwind or roll back signal writes. The async bridge disables an instance after a trapped poll or thrown host import. A normal procedure rejection is recoverable and drops its suspended future.

## 9. Share allocator code, isolate heaps

Allocating bundles import allocation and deallocation from one content-hashed Wasm file:

```text
                    allocator.wasm download + compiled module
                              /                 \
                      allocator instance A   allocator instance B
                              |                 |
                      page A + memory A      page B + memory B
```

The browser shares downloaded/compiled code. Each page module gets a separate allocator instance and memory. Pointers are local to that memory and must not cross page instances.

Within one allocating bundle:

```text
linear memory
  first 64 KiB: allocator data and allocator stack
  following:   page data, page stack, and heap space
```

The build redirects wee_alloc's allocation methods after wasm-bindgen, before optimization. It checks method signatures, memory boundaries, and remaining allocator helpers. Page futures and destructors stay in the page module. Page function pointers never enter the allocator's function table.

A safe Rust retention function keeps the allocator implementation available for extraction:

```rust
pub fn retain_allocator(size: usize) {
    let mut bytes = alloc::vec::Vec::<u8>::new();
    let _ = bytes.try_reserve_exact(size);
    core::hint::black_box(bytes);
}
```

Fallible reservation avoids retaining allocation-error formatting. The retention function is removed from the final provider. Only allocation and deallocation are exported. Application allocation failures still follow the page's abort path.

Nonallocating release bundles do not load the allocator. Its small-allocation pools remain enabled; disabling them caused memory growth in stress tests. Passing those tests does not establish safety for every long-running allocation pattern.

## 10. Remove overhead at the right stage

### Inline generated event helpers

An input handler needs one field:

```rust
$(|e: Event| query.set(e.target.value))
```

If the helper is inlined too late, event construction and cleanup can survive optimization. We add the hint to generated synchronous event helpers:

```rust
#[cfg_attr(not(debug_assertions), inline(always))]
pub fn expr_input(query: &Signal<Text>, e: Event) {
    query.set(e.target.value);
}
```

LLVM can then remove unused event fields. User source needs no annotation. Async helpers are excluded. Menu fell from 929 to 358 bytes.

### Remove memory only when all of it is unused

After event cleanup, menu needs no linear-memory operations. The trim pass checks:

```text
known scalar/JS-value imports only?
generated JS does not access memory?
only known internal memory/global exports?
Binaryen can remove the entire memory?
    yes -> keep the trimmed module
    no  -> keep the original bytes
```

This reduced menu from 358 to 262 bytes. It preserves memory used by loads, startup functions, indirect calls, and bulk-memory operations.

Drink still contains diagnostic strings mixed with live tables. We leave that data intact. Removing diagnostic code alone does not prove its containing data segment is dead.

### Measure compressed output too

The last optimization pass was applied one change at a time. Each retained change passed an adversarial review of behavior and developer experience.

| Change | Raw result | Decision |
| --- | --- | --- |
| Fallible allocator seed and metadata cleanup | Allocator 3,241 -> 1,836 B | Keep |
| Inline generated synchronous event helpers | Menu 929 -> 358 B | Keep |
| Remove proven-unused memory/data | Menu 358 -> 262 B | Keep |
| Reuse task destructor for cancellation | Drink 2,448 -> 2,302 B | Keep |
| Typed import per procedure | Drink 2,302 -> 2,282 B | Reject as default |

The rejected procedure import had this shape:

```javascript
function start_order(name, quantity) {
    procedure_start(0, [name, quantity]);
}
```

It reduced Wasm argument setup but added imports and JS adapters per procedure. Combined JS + Wasm Brotli size saved 18 bytes for coffee and grew by 198 bytes for a 12-procedure fixture. The shared argument-array bridge remains.

Changing Rust's optimization level was also unhelpful for drink: `"s"` produced 2,773 bytes and `"z"` produced 2,962 bytes against a 2,448-byte baseline at that point in the experiment.

## 11. Limits and next work

| Area | Current limit |
| --- | --- |
| Dependencies | Same-crate items and supported standard-library operations; no arbitrary external-crate extraction |
| Source extraction | Copies supported source; cannot generally reproduce macro expansion or target-specific configuration |
| Types | Several generic forms, manual trait implementations, and custom destructors are unsupported |
| Boundary | No struct/collection/String payloads, wider integers, pointer-sized integers, or f32 |
| Async | Topcoat procedures only; one pending operation per handler |
| Bundling | Visible same-crate component references; shared component code can be duplicated |
| Loading | Example loader is eager; page/shard bundle ownership is not yet a complete lazy-loading product |
| Linker | Pinned allocator/binding assumptions; no general Rust dynamic linking |

Useful next steps:

- Make extraction diagnostics point clearly to unsupported application code.
- Add real route-driven lazy loading and test shared component entries across bundles.
- Track raw, compressed, and total JS + Wasm sizes across more applications.
- Investigate safe data liveness for drink's remaining static data.
- Run the JS framework benchmark workload and compare with Topcoat's main JS backend. No performance conclusion follows from these size results alone.

## 12. Reproduce and inspect

See [the compiler driver guide](tools/topcoat-split/docs/split.md) for toolchain installation and environment overrides. From the repository root, with those dependencies installed:

```sh
export WASM_OPT="$PWD/tools/topcoat-split/node_modules/.bin/wasm-opt"

cargo build -p topcoat-cli --bin cargo-topcoat
node tools/topcoat-split/examples/coffee-shop.mjs --release

TOPCOAT_WASM_PUBLIC="$PWD/target/coffee-shop-wasm/public" \
    PORT=8083 target/topcoat-native/server/debug/coffee-shop
```

In another terminal:

```sh
COFFEE_URL=http://127.0.0.1:8083 node target/coffee-shop-wasm/checks.mjs
node tools/topcoat-split/examples/async-checks.mjs
node tools/topcoat-split/examples/bridge-checks.mjs
node tools/topcoat-split/examples/trim-checks.mjs
```

Use `--debug` for readable client artifacts. Restart the server when switching profiles. Both profiles share the server executable.

Checks cover real HTML hydration, signal isolation, Unicode, integer bounds, HTTP procedures, sequential/concurrent handlers, rejection and retry, stale/foreign task handles, reentry, allocation failure, alignment, and memory reuse. Debug and release paths are exercised.

| File or directory | Purpose |
| --- | --- |
| [Native expression expansion](crates/topcoat-runtime/grammar/src/expr/native.rs) | Mark roots and bind manifest captures |
| [Compiler driver](tools/topcoat-split/src/main.rs) | Run extraction after server type checking |
| [Extractor](tools/topcoat-split/src/split.rs) | Find owners, captures, and dependencies |
| [Client code generation](tools/topcoat-split/src/wasm.rs) | Build runtime adapters and dispatchers |
| [Browser bridge](tools/topcoat-split/src/wasm/bridge.js) | Capture frames and task scheduling |
| [Host imports](tools/topcoat-split/src/wasm/host.js) | Signals, Text, events, and procedure transport |
| [Client build](tools/topcoat-split/build-client.mjs) | Compile, bind, link, optimize, and rename |
| [Allocator linker](tools/topcoat-split/allocator/split.mjs) | Split allocator code and connect memory |
| [Unused-memory pass](tools/topcoat-split/trim.mjs) | Conservative whole-memory removal |
| `target/coffee-shop-wasm/crates/` | Generated Rust and manifests |
| `target/coffee-shop-wasm/sizes.json` | Current bundle measurements |
| `target/wasm-reductions/` | Per-step measurements and rejected procedure prototype |

The `target/` artifacts are local outputs and can be deleted by a clean build. The size tables above preserve the report's measured results.
