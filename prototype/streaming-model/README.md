# streaming-model

A hand desugared prototype of the component future model from `DESIGN.md`, built to validate the strategy before any macro work. Everything `#[component]` and `view!` would generate is written by hand: a component is an async fn that runs its body once, then loops over renders separated by a pass boundary, with children stored as futures in the parent's own frame. A manual driver runs passes deterministically; tests control every piece of I/O through explicit triggers.

Run it with `cargo test` in this directory. The crate is standalone and not a member of the framework workspace.

## What the tests prove

- The load bearing borrow pattern compiles and works: a keyed collection of `Pin<Box<dyn Future + 'f>>` child futures that borrow the parent's body locals, held across `.await`, with entries inserted on later passes. This was the checkpoint that could have killed the design. Covered by the straw man test (child prop borrows the parent's `String`), the two level nesting test, and the `Ready(&T)` test (card props borrow the deferred output).
- Bodies run once, renders run per pass, with zero memoization anywhere. The un-annotated fetch footgun does not exist in this model.
- A settled tree advances in a single poll per pass.
- Sequential chains peel one layer per pass: a component born from a `Ready` arm runs its body then and registers its own defer.
- Passes see a consistent snapshot: a deferred future that completes mid pass is not observed until the next pass.
- Errors are future completion. A deferred `Result` propagated with `?` on a later pass unwinds the subtree, drops it in compiler managed order, and is caught by the layout's slot; siblings outside the slot survive. Both the inline case (error during an advance) and the stashed case (error while the tree is suspended, consumed on an automatic extra pass) are covered, plus birth failure, whole page failure, and handling the error at the source by matching `Ready(Err)`.
- Eviction is a drop. A component orphaned by a branch switch is swept and dropped; a `Drop` impl that reads a borrowed prop runs soundly both at eviction and at end of request, because frame drop order guarantees children drop before the locals they borrow. This is the teardown case a shared arena cannot offer in safe code.
- Deterministic renders produce byte identical slots, so an unchanged component contributes nothing to a pass's changed set: the premise of boundary diffing.
- Compile time rejections hold, checked as `compile_fail` doctests in `src/lib.rs`: `.await` in a render position (a closure that is not async) is E0728, a child prop borrowing a per pass temporary is E0597 at the invocation site, and a deferred future borrowing a body local is E0597 at the spawn.

## Findings for the design document

- The `Defer` handle pattern fell out of the desugaring on its own: the handle is created in the body, holds the output slot in the component's frame, and the render observes it per pass. `Ready(&T)` is then a plain borrow of frame storage, with no global cache, no `Any`, and no arena. This supports moving the design's `defer` API toward a body created handle polled from the view.
- Storing a `'cx` bounded deferred future is a real knot: the context cannot own futures that borrow it (the type would be self referential and dropck rejects it), so the handoff from render code to the driver needs the task scoped channel the pass protocol open question describes. The prototype sidesteps it by bounding deferred futures with `'static` and passing inputs owned, which is also what the implicit memoize leaning produces in practice.
- A catching layout needs to remember that its slot failed (a frame local here), or the next pass would rebirth the slot and loop the failure. The macro generated slot handling has to encode that state.
- Duplicate keys are cheaply detectable at runtime: advancing the same key twice in one pass is an assertion in the prototype and should be a proper diagnostic in the framework.

## Simplifications

- Deferred futures are `'static`, per the finding above.
- Single threaded: `Rc` and `RefCell` throughout, so `Send` propagation and its diagnostics are not validated here. The borrow patterns themselves are `Send` agnostic.
- Output assembly substitutes child markers recursively instead of streaming chunks, and the changed set stands in for boundary hashing; the wire format is out of scope.
- Identity is an explicit per invocation key instead of the identity system; the two are interchangeable for what this validates.
