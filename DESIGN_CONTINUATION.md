# The Continuation Model

A sequel to DESIGN.md. That document built a reactive system out of reactive
expressions, `live` constructs, cells, tenders, and frame sets. This document
examines a reframing: the whole system is continuations. A component is an async
generator of views, `render!` yields one, and the reactive machinery is nothing
but nested generators resuming. The verdict up front: the intuition is correct,
the previous design is implementable on top of this primitive, and most of its
machinery dissolves into it.

## The Primitive

One concept carries everything. A `ViewHandle` is a view continuation: the
non-`'static` future of a component body plus a yield slot, built the way
`async-stream` builds streams, no unsafe required. It is NOT boxed by default:
a component fn returns an opaque `impl` type, and callers that drive children
hold them unboxed in their own frames, the way today's expansion holds unboxed
render futures in `try_join!`. Boxing exists only where a handle must be
reified to a single runtime type, which in practice may be only the root the
driver holds (and recursive components, as `#[component(boxed)]` covers
today). That restores today's zero-allocation-per-component profile without
the old design's deferred unboxing work. The one operation is the stream
operation:

```rust
impl ViewHandle<'_> {
    /// The next complete view, an error, or `None` when closed.
    pub async fn render(&mut self) -> Option<Result<View>>;
}
```

`#[component]` transforms an `async fn ... -> Result<View>` into a function
returning `ViewHandle<'_>`. `render!` yields a view and resumes; falling off the
end, or a `?`, yields the final `Ok` or the `Err` and closes. `View` is
unchanged: the fully realized, inert output, no continuation inside it.

A consequence: `ViewHandle` is NOT `Clone`, unlike DESIGN.md's cheap index
into a shared cell. The handle holds the actual continuation state, so it is
an owned, move-only value with exactly one driver. Sharing rendered content
never needs handle clones anyway: the shareable thing is the slot-backed
`View` a handle's output fills, which is inert and cheap to embed in many
places. This deletes the consumer count, the clone-sharing rules, and the old
two-`live`-constructs sharing question outright.

This is DESIGN.md's `Reactive` trait arrived at from the other side. The
one-method contract `async fn next_state(&mut self) -> Option<Result<View>>` and
`render()` are the same signature; the difference is that the old design treated
the state stream as something a special value produces and a special construct
consumes, while here the stream is what a component is. The convergence is
strong evidence the model is right.

## What the Old Building Blocks Become

**`defer` plus `live match` become sequential yields.** The state machine was
only ever encoding "show this, then that":

```rust
render! { <p>"skeleton"</p> };
let drinks = drinks(cx).await?;
render! { <p>(drinks)</p> }
```

Laziness holds because generators do not run until polled. Fires-at-most-once
holds because control flow is linear. The `Deferred` enum, the `live` keyword,
`run_node`, and the reactive-expression concept are not needed as primitives;
`live match defer(f)` can be kept as pure sugar for exactly the code above.

**Reactive nodes become nested sub-continuations.** A `rust { ... }` block
inside a view is an inline anonymous component: its own generator, embedded by
reference in the surrounding output. The old node loop, arms-as-frames, and the
arms-are-`Fn`-closures ownership rule all disappear, and with them the rule's
user-facing cost: successive renders are ordinary linear code, so moving a value
into a later render is just Rust, no `.clone()` discipline needed except for
values shared across yields.

**Multi-fire expressions become loops.** What needed the `run_node` race and the
signals refinement is a `while let` over a stream, with cancellation as an
ordinary `select` if wanted. This is strictly more expressive than the old
model: user code can sequence, loop, branch, and hold state between renders,
none of which the arms model allowed.

**Error handling is the stream contract.** A body that fails before its first
`render!` yields one `Err` and closes: the old pre-fill failure. A body that
fails after yielding: the old error transition. Catching is consuming a child
handle and matching on what it yields; rethrow is `?` on the yielded `Result`.
Bubble mode and catch mode survive; the layout default becomes the slot form
below, with `live match`-style catching as the opt-in handle form.

**View arguments are slots, correcting DESIGN.md.** In
`outer_component(child: inner_component())` the receiver is NOT handed a
continuation to drive. The caller reserves a slot, keeps driving
`inner_component`'s handle itself, and passes `outer_component` the slot-backed
placeholder `View`. This is how child props render today, and it is the better
semantics for two reasons: the outer and inner components render concurrently
(neither waits on the other's first yield), and when the inner component
re-yields, the outer component is NOT resumed or re-rendered; the caller
refills the slot and the outer's already-rendered output updates through the
buffer indirection. Layouts work the same way: the framework drives the page's
continuation and refills the layout's slot; the layout body runs once. A
receiver that wants to observe or catch a child's stream opts in by taking a
`ViewHandle<'_>` prop explicitly and driving it in a `rust` block, which is
the catch mode above; the slot form is the default.

## A Superset of Today's API

The model contains today's API as its degenerate case, which makes it a clean
superset rather than a migration. The rule: a component body's final
expression, an inert `view! { ... }` in tail position or an explicit
`return view! { ... }`, is the FINAL render. The handle yields it and closes.
A body containing no `render!` therefore compiles and behaves exactly as
today: one yield, then closed, and driving that single-yield stream to
completion is precisely today's render. The declared `Result<View>` return
type becomes literal again: the body really does return its last view, and
the stream a handle exposes is zero or more `render!` yields followed by
exactly one final `Ok` or `Err`, then `None`. A trailing `render! { ... }`
with nothing after it is equivalent to the same view in tail position: the
last render is the final one however it is spelled.

Everything else in today's surface carries over for the same reason: inert
`view!` values bound to locals keep meaning what they mean today, and child
props stay slot-backed `View`s per the correction above, so existing
component signatures do not change. Layouts move wholesale to the same slot
form as any `child`-taking component: today's `slot: Result<View>` implied
the page's final state was known when the layout ran, serializing the layout
behind the page, while the slot form runs both concurrently. Which collapses
the concept: a layout is just a component that the router automatically wraps
around a page by path prefix. `#[layout]` contributes route registration and
nothing else; the body is an ordinary component with a slot-backed `child`,
and a layout that wants to catch the page's errors opts into the handle-prop
form exactly like any other wrapper component.

## The Machinery That Remains

The model is not machinery-free; it relocates the machinery into one place, the
handle's poll. Three obligations from DESIGN.md do not dissolve and must live
there:

- **Concurrent children.** A caller that starts children must drive its own
  generator and every child concurrently, or siblings serialize. With unboxed
  handles this is not a runtime list of boxed entries: children whose count is
  known at expansion time join structurally, today's `try_join!` shape,
  polled inline by the caller's generated code; a `Vec` of one homogeneous
  type covers loops, as today. The `RefreshSet` dissolves into these static
  joins, and a dynamic child collection exists only where boxing does.
- **Re-yield propagation.** A child component's new yield refills its slot, so
  every ancestor's already-yielded `View` updates through the buffer
  indirection with no ancestor resuming; only the driver needs to learn that
  something changed. Conceptually the driver is just
  `while let Some(view) = root.render().await { send it }`; how it actually
  observes changes (dirty tracking and the rest) is deliberately left open
  here, with many details still to figure out.
  The rule still needing statement is for a handle's OWN stream: a `render!`
  whose body contains `rust` sub-continuations yields once when every one has
  yielded its first view (the old barrier and first-paint semantics), then
  either re-yields per sub-block yield or, consistently with the slot
  semantics for children, lets sub-blocks refill slots too; which of the two
  is an open question below.
- **Block reuse.** A naive re-yield rebuilds the subtree's blocks; the old
  design re-rendered only the changed slot. The same instruction-buffer
  indirection covers it, embedding children by slot reference so a re-yield
  reuses unchanged blocks, and `render_into(dst)` is the natural optimization
  surface. The stage 1/2 buffer work (slots, refill, dirty) carries over nearly
  intact.

Cells, tenders, tickets, frame ids, the consumer count, the spliced list,
`invoke`/`adopt`/`splice` as distinct operations, and the `LiveState` slab all
collapse into the handle owning its (unboxed) future and its yield slot, with
per-callsite static joins replacing every runtime collection of entries. The dropck
findings stop applying, because there is no set borrowing later locals: the
borrows live inside one generator frame. Cancellation stays structural and gets
simpler: dropping a handle drops the continuation and every child, which is
exactly the frame-drop-only decision.

## Can DESIGN.md Be Implemented on Top? Yes.

| DESIGN.md                              | Continuation model                                   |
| -------------------------------------- | ---------------------------------------------------- |
| `defer` + `live match`/`if let`        | sugar for yield, await, yield                        |
| Reactive expression / `Reactive` trait | the handle itself; `render()` is `next_state`        |
| Reactive node, arms, `run_node`        | a `rust { ... }` sub-continuation                    |
| Signals (future)                       | `while let` over a stream in a `rust` block          |
| Bubble / catch / rethrow               | splice / drive-and-match / `?` on the yielded result |
| Layout `slot: ViewHandle`              | an ordinary component with a slot-backed `child`     |
| View arguments                         | slot-backed `View` by default; handle prop to catch  |
| `settled` (punted)                     | an adapter awaiting child streams' completion        |
| Frame-drop cancellation                | dropping a handle                                    |
| First-paint barrier                    | first-yield rule above                               |
| `boundary`, wire format (out of scope) | unchanged: diff successive yielded views             |

Nothing in the old guide is inexpressible, and several punted or awkward pieces
(`settled`, multi-fire, ownership rules) become easy or moot.

## Open Questions for This Model

- The yield mechanism's exact shape: `async-stream` style thread-local yield
  slot versus threading a sink argument through the generated body, and keeping
  the handle `Send` and free of unsafe either way.
- Whether a `rust` sub-block's later yields re-yield the enclosing handle's
  stream (the twice-yielding outer `render!`) or refill a slot like child
  components do; the slot form is more uniform and avoids resuming ancestors,
  but the re-yield form makes a handle's stream self-describing.
- Where the error stream of a slot-driven child surfaces: the driving caller's
  frame by default (bubble), with the handle-prop form as the catch; the exact
  layout catch spelling under slot-by-default, now that layouts are ordinary
  wrapper components: how the router hands the catching form the page handle.
- Sharing content across successive yields of the same body (the old
  shared-feed pattern): whether a spliced child persists across the parent's
  own re-renders by identity or by an explicit re-splice of its slot-backed
  `View`. Handle clones no longer exist, so the `View` is the only shareable
  form.
- `rust { ... }` as surface syntax versus reserving it and shipping only the
  `live`/`defer` sugar first.
- How much of the landed stage 1/2 implementation survives: the buffer work and
  layout/router changes largely do; the grammar's live-node emission would be
  rewritten around generator lowering; the `RefreshSet` public surface would
  shrink or hide.

## Verdict

The continuation model works, and it is the better primitive. It replaces a
state-delivery architecture (values that change, constructs that react) with a
control-flow architecture (code that keeps going), which is the same shift
DESIGN.md itself made against Suspense, taken to its conclusion: the live render
was already one big continuation, and the cell/tender/set machinery existed to
fake resumption for pieces of it. Making resumption real removes the fake. The
costs are concentrated and known: the yield plumbing, the child-driving poll,
and the re-yield/reuse rule. Everything else in DESIGN.md becomes either sugar
or a library adapter on one type with one method.
