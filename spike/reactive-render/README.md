# reactive-render spike

A throwaway, macro-free, dependency-free, `#![forbid(unsafe_code)]` crate
that hand-writes the code the `view!` and `#[component]` macros would
generate under DESIGN.md, so the compiler can judge the shapes and the
driver can prove the streaming behavior. `cargo run` prints two
scenarios: a happy path streaming four chunks, and a mid-stream error
transition climbing to the root.

Each module documents the Topcoat-level API it checks in its doc
comments:

- `src/scope.rs`: the render scope (thread-local install, `&mut` access,
  `Send` with no locks), views and slots, cells, per-frame ticket
  counters.
- `src/set.rs`: `RefreshSet` (`invoke`/`adopt`/`splice`/`push`/`ticket`/
  `barrier`/`run`), the tender, `ViewHandle`, the one-method `Reactive`
  contract, `Defer`, and `run_node`.
- `src/page.rs`: the hand-expanded page: fused child with borrowed
  props, a reactive node per `live match`, arm frames, the unfused
  shared-feed handle spliced into both arms, the arm `?` rethrow.
- `src/main.rs`: the driver: one task, scope installed per poll,
  sweep-until-quiescent, first paint plus one chunk per dirty pass.

## What the spike verified as designed

- The whole render is one `Send` future reaching shared state with no
  `Arc`, `Mutex`, atomic, or stored `Waker` (asserted in `main`).
- Sweep-until-quiescent needs no wakers: cross-frame fills (the feed's
  tender in the body frame, its waiter in an arm frame) are observed on
  the next pass.
- Laziness: `Defer`'s first poll happens at consumption; a parked tender
  runs only once spliced, and resolves as a no-op if every handle clone
  drops first.
- The shared-feed pattern: one render, clones spliced into both arms,
  splice-after-delivery served from the cell's cache, the arm swap
  neither cancelling nor restarting it, and the feed's own later swap
  landing inside the arm that replaced the one it first appeared in.
- `run_node`'s race in safe code: a new state drops the arm frame, which
  cancels the replaced subtree's work; retirement (`None`) lets the arm
  finish its remaining live work.
- Error transitions: an arm's `?` (the `orders?` failure) fails the
  node, climbs the frame tree through `run`, and surfaces at the root,
  all after the first paint went out.
- `'frame` props lifetimes: fused children borrow the caller's body
  locals, and the component future is generic over the invoking frame,
  not `'cx`.

## Findings: where DESIGN.md's sketches needed correction

1. **Tail-position epilogues do not compile.** Temporaries of a block's
   tail expression outlive the block's locals, so
   `__refresh.run().await` in tail position trips drop analysis. The
   generated epilogue must bind: `let __done = __refresh.run().await;
   __done`.
2. **Set-before-locals does not compile; the set lives in an inner
   block.** Drop analysis is not path-sensitive: any `?` between a push
   and the epilogue (the barrier's `?`, a body `?`) leaves a path where
   the set drops in scope while its entries borrow later locals, and
   that is rejected regardless of the happy-path `run(self)` move. The
   working shape: body locals in the outer scope, the tail `view!`
   expansion as an inner block owning the set. Mid-body
   `let x = view! { ... }` bindings need a pending-adopt desugar: the
   future is created at the binding (borrowing only earlier locals) and
   registered when the tail expansion's set exists.
3. **The arms closure cannot be an `async` closure.** `AsyncFnMut`
   cannot carry a `Send` bound on its returned future, and pushed node
   futures must be `Send`. The working shape is a plain `FnMut`
   returning a `Send` future, which forces every capture to be a
   per-call copy: `Copy` slots, `Copy` tickets, `Copy` references to
   body locals, and handle clones. This is also why the ticket's done
   flag lives on the scope rather than in the `Ticket` value.
4. **Arm closures capture handles by clone, not by reference.** A
   `&handle` capture ties the entry to a local that cannot outlive the
   set; a clone per call is exactly what cheap `Clone` handles exist
   for.
5. **`ViewHandle`'s `Drop` must tolerate a missing scope.** The root
   future (and every handle in it) is dropped by the driver outside any
   poll, when nothing is installed; consumer accounting no longer
   matters then, so the drop hook uses a tolerant `try_with`.
6. **Fused cells do not belong in the `spliced` list.** Only tenders
   write the terminal completed state; a fused invocation's future is an
   entry of the same frame, tracked directly. Adding fused cells to
   `spliced` livelocks `run` (caught here at runtime).

## What the spike does not cover

Component handles consumed by `live match` (catch mode), the layout
slot, `boundary` markers and per-chunk diffing, loop batching, and the
blocking mode for `view!` outside a transform. All are additive over the
shapes proven here.
