# Implementation Delta

Where the landed implementation (stages 1 and 2, uncommitted on this branch)
deviates from DESIGN.md. DESIGN.md is left as designed; this file tracks the
differences until they are either accepted into the design or the code is
changed to match. Out of scope in the implementation, as agreed: `boundary`,
boundary hashing/diffing, the wire format, the swap script, and HTTP chunk
streaming (the router drives the live render to completion instead).

## Guide-visible deltas

- **Trailing child content stays a plain `View`.** Only NAMED view-valued
  arguments are `ViewHandle<'_>`, written as `view!` blocks in argument
  position (`title: view! { ... }`). Trailing children compile synchronously
  into the caller's frame through the existing `child: View` convention, which
  kept the whole registry unmigrated. DESIGN.md's `card` example shows
  `children: ViewHandle<'_>`; upgrading children to handles is a possible
  later step.
- **Component calls in named-argument position are not auto-adopted**
  (ambiguous with plain fn calls). Wrap them: `title: view! { some_comp() }`.
- **View-level `let` locals cannot be borrowed by live work** (invoke props,
  defer futures, live arms): the frame's set precedes the view's statements
  and drop-order analysis rejects the borrow. Body locals work as designed;
  move the `let` to the body.
- **`view!` in nested expression positions and `view! { ... }?` bindings fall
  back to blocking (completion) semantics** instead of being a compile error.
  Pending-adopt is implemented only for direct `let x = view! { ... };`
  statements of a body. `live`/`defer` outside a
  `#[component]`/`#[page]`/`#[layout]` transform is a compile error
  ("`live` needs a live render: ...").
- **`defer` recognition rule**: in a live scrutinee position, a call whose
  path ends in `defer` with exactly one argument is rewritten to
  `topcoat_view::live::defer(__cx, argument)`. Anything else is untouched.
- **Live-match scrutinee grammar**: `defer(...)` is a deferred load, another
  `path(...)` is a component invocation adopted as a handle, anything else is
  a plain expression that must implement `Reactive` (bind other reactives to
  a local first).
- **Keyed identity works and derives before the props build**
  (`Identity::keyed_invocation`), so `drink_card(key: &drink.slug, drink:
  drink)` compiles.

## Machinery deltas

- **Arms are not `FnMut` closures.** The macro cannot enumerate the locals
  arm bodies reference, so the emitted node inlines the loop as a plain
  non-move `async` block over the public `race_node` helper. Non-move blocks
  borrow body locals for free and reject consuming them, which is exactly the
  guide's borrow-not-move ownership rule. `run_node` remains as the
  hand-written/test entry. Each arm compiles as its own complete frame
  expression; the frame prelude (frame id, node slots, tickets) registers
  before the set so entries never borrow post-set locals.
- **`Component::Props` is a generic associated type** (`type Props<'frame>`)
  on a non-generic marker, so a layout's borrowed slot prop and the router's
  `From<marker>` registration coexist. Render signature otherwise as
  sketched: `render(self, cx, props, fill) -> impl Future<Output =
  Result<()>> + Send + 'frame`.
- **No `RenderScope` struct**: `ViewBuffer` plays that role; the live state
  is a `LiveState` field on it (cells as an append-only `Vec`, `usize`
  newtype ids; `u32` and generation bits deferred).
- **Errors are not `Clone`** (anyhow-backed), so cell errors are
  taken-once: the first consuming frame gets the real error, any other
  consumer of the same failed cell gets a `SplicedViewFailed` fallback.
  Retirement is a persistent `failed` flag.
- **`post_error` does not set `dirty`**: dirty strictly means the document
  changed; an uncaught error surfaces as the root `Err` instead of a chunk.
- **The blocking mode keeps the entire old emitter** (try_join/Either/loop
  joins) with invocations wrapped in `internal::complete`; the root scope
  role sweeps until quiescent so blocking `view!` completes under any
  executor.
- **`internal::Own<T>`** forces by-value capture of the reactive value and
  each state in the emitted node loop (RFC 2229 otherwise downgrades
  by-value uses of `Copy` types to by-ref captures, which breaks the loop).
- **Handle interpolation `(handle)`** dispatches by type at hoist via
  autoref (inherent method on `ViewHandle` beats the blanket `Interpolate`
  fallback), because a splice is frame bookkeeping that must precede the
  barrier.
- **Spike finding 1 (bound epilogue) was not needed** in the landed shapes:
  tail-position `refresh.run().await` compiled everywhere. The emission
  keeps the binding defensively anyway.

## Where the implementation lives

- Runtime: `crates/topcoat-view/src/live/` (refresh set, handles, reactive
  contract, node loop, test support with the channel reactive and the
  `LiveRender` driver) and `crates/topcoat-view/src/buffer/live_state.rs`.
- Emission: `crates/topcoat-view/grammar/src/view/` (live constructs, live
  node HIR, blocking/live emitter split) and
  `crates/topcoat-view/grammar/src/component/body.rs`.
- Router: `crates/topcoat-router/src/page.rs` composes page and layouts into
  one live render and drives it to completion; layouts take
  `slot: ViewHandle<'_>`.
- Tests: `crates/topcoat-view/tests/live.rs` (runtime),
  `crates/topcoat-view/tests/live_macro.rs` (emission),
  `crates/topcoat-router/macro/tests/layout.rs` (layout catching and
  rethrow).
