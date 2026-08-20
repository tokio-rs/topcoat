# Streaming SSR, futureless

This document proposes a smaller alternative to the `defer` primitive in [DESIGN.md](DESIGN.md). Render passes, diffing, and the wire format stay as designed there; only the primitive changes, and with it the whole question of how deferred futures are stored.

The observation: only the first paint has a deadline. Once the skeleton chunk is out, a later pass can await anything inline, because whenever it finishes, the result streams down the open response. The framework never needs to hold a future. It only needs to know where to cut the first paint.

## The primitive

```rust
/// Returns `true` on the first pass that reaches this call site,
/// `false` on every later pass.
#[track_caller]
pub fn defer(cx: &Cx) -> bool
```

The name is a placeholder and a bad one; see [Naming](#naming).

```rust
use topcoat::{Result, context::Cx, view::{component, defer, view}};

#[component]
async fn drink_grid(cx: &Cx) -> Result {
    if defer(cx) {
        return view! {
            <div class="grid gap-4 sm:grid-cols-2">
                for _ in 0..6 {
                    <div class="h-32 animate-pulse rounded-lg bg-muted"></div>
                }
            </div>
        };
    }

    let drinks = drinks(cx).await?;
    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            for drink in drinks {
                drink_card(key: &drink.slug, drink: drink)
            }
        </div>
    }
}
```

There is no `Deferred` enum, no wrapped future, and no closure. The skeleton pass returns before the fetch is ever expressed; the next pass takes the other branch and awaits inline, with plain ownership, plain borrows, and a plain `?`.

## Render passes

A call site is identified by the component identity of the enclosing body combined with the call site via `#[track_caller]`, so two instances of the same component defer independently and the usual `key:` rules apply inside loops. The request keeps one bit per site: whether it has been reached before.

1. The page renders. Every `defer` at a new site records the site and returns `true`; the caller renders its skeleton and never reaches its data loading.
2. The completed HTML ships as a chunk. If no site returned `true`, the response is complete; a page that never defers renders exactly as today.
3. Otherwise the page immediately renders again. Sites seen before return `false`, so their bodies await their data inline. Sibling components still render concurrently, so a pass overlaps all of its loads and completes when the slowest one does.
4. Steps 2 and 3 repeat. Each pass that reaches a new `defer` site adds one more pass; a pass that reaches none is the last.

Nothing is stored across passes except the set of seen sites. There is no future to keep alive, no result to hand back on a later pass, and no call-site reconnection.

## Errors

There is nothing to design. Every await is an ordinary await on a live call stack, on every pass, so `?` bubbles through components into layouts exactly as today. The mid-stream caveats from DESIGN.md (status code and headers are already sent, redirects become swap instructions) apply unchanged.

## Skeletons above the data

The marker sits where the skeleton is drawn, not where the data loads. Components below it stay ordinary async components that know nothing about streaming:

```rust
#[component]
async fn dashboard(cx: &Cx) -> Result {
    if defer(cx) {
        return view! { <div class="h-96 animate-pulse rounded-lg bg-muted"></div> };
    }

    view! {
        <div class="grid gap-6 lg:grid-cols-3">
            revenue_chart()
            top_customers()
            open_invoices()
        </div>
    }
}
```

One `defer` batches all three sections behind one skeleton. Moving a skeleton up or down the tree, or splitting one into three, changes nothing about how the data loads and requires no error type to smuggle pendingness up the call stack.

## Layers

A `defer` first reached on a later pass defers then, so sequential loading chains cost one pass per layer:

```rust
#[component]
async fn post_page(cx: &Cx) -> Result {
    if defer(cx) {
        return view! { post_skeleton() };
    }

    let post = post(cx).await?;
    view! {
        <article>
            <h1>(&post.title)</h1>
            (&post.body)
        </article>
        comments(post: &post)
    }
}

#[component]
async fn comments(cx: &Cx, post: &Post) -> Result {
    if defer(cx) {
        return view! { <p class="skeleton">"Loading comments..."</p> };
    }

    let comments = comments_for(cx, post).await?;
    view! {
        for comment in comments {
            comment_card(key: &comment.id, comment: comment)
        }
    }
}
```

Pass 1 ships the post skeleton. Pass 2 loads the post and ships it with a comments skeleton. Pass 3 loads the comments.

Note that `post(cx)` runs on pass 2 and again on pass 3. `#[memoize]` makes the second run free; without it the query runs twice. This is the one place the memoize discipline from DESIGN.md survives: it matters only on pages with more than one layer, because a single-layer page runs its loads exactly once. A dev-mode check can catch the mistake precisely: a load that completed on pass N and suspends again on pass N+1 is unmemoized by definition.

## What this removes

Compared to the `defer(cx, future)` of DESIGN.md:

- No future storage. The open question of how to run the work once and produce the value again on later passes (spawning, `'static` bounds, `T: Clone`, leaning on `#[memoize]`) disappears; the work simply is not reached on the skeleton pass.
- No `Deferred` enum and no keyed defer variant beyond the ordinary identity rules. The per-site state is one bit.
- No memoize requirement for single-layer pages, which are the common case.
- No leaked work from discarded views: a view that never joins the page registered nothing.
- Blocking mode is the primitive returning `false` unconditionally: one render, every await inline, byte-equivalent to the streamed page's final state. That is the mode for crawlers, tests, mail bodies, and views rendered to strings.

## The trade

A pass completes when its slowest await completes, so every skeleton of one layer resolves at once. Under DESIGN.md's future tracking, each region swaps as its own data arrives; here, a 200ms query and a 3s query behind two sibling skeletons both swap at 3s. For the common page with one slow region there is no difference. Recovering per-region completion is possible without changing this design's surface, by shipping parts of a pass as their subtrees finish rendering, and is left out of scope here.

The slow load also starts later: at the top of pass 2, after the fast parts of pass 1 finished, instead of concurrently with them. The loss is roughly the duration of the first pass. If it ever matters, an optional prewarm hint that starts a memoized load during the skeleton pass would recover it.

## Naming

`defer` is wrong: nothing is deferred, and no data or future is involved. The call answers one question, whether this cut point has shipped yet. The name should stay generic rather than describe skeletons or paints, since the pending branch can render anything. Candidates in that spirit: `pending(cx)`, `later(cx)`, `partial(cx)`. Undecided.

## Open questions

- **Limits.** A cap on passes or a deadline per request, after which pending sites keep their skeletons, same as DESIGN.md.
- **Prewarm.** Whether to offer the hint described above, and its shape.
- **Per-region flushing.** Whether and when to ship completed parts of a pass early, and what that needs from the renderer.
- **Diagnostics.** The dev-mode unmemoized-load check, and a warning for a site whose identity is ambiguous inside a loop.
