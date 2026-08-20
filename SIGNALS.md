# Signals v2

This document proposes a redesign of signals on top of the render-diff-ship machinery from [the streaming SSR proposal](https://github.com/tokio-rs/topcoat/blob/ssr-proposal/DESIGN.md). It is the concrete form of that document's "Signals Without Shards" section, plus the answer to what becomes of `#[shard]`.

Three changes:

1. The `signal` statement in `view!` becomes a plain Rust function, callable anywhere in a component body or composed in a function.
2. Reading a signal in ordinary server code registers it as a dependency of the render. When it changes in the browser, the client refetches the page with the current signal values, the server re-renders with those values filled in, and the boundary diff ships only what changed.
3. `#[shard]` stays, but its meaning shifts: it is an upper bound on re-rendering. A signal whose server reads all sit inside a shard re-renders only that shard, not the page.

## The primitive

```rust
/// Creates a signal, or resumes one: if the request carries a value for
/// this signal's identity, that value is used and the closure never runs.
pub fn signal<T>(init: impl FnOnce() -> T) -> Signal<T>
```

A signal's identity is the component identity of the enclosing body combined with the call site via `#[track_caller]`. The identity is stable across renders and across requests, so no hook-ordering rules are needed and the usual `key:` rules cover signals reached through loops. This replaces today's random per-render `SignalId`.

`Signal<T>` is a cheap handle; the value lives in a request-scoped store. Handles pass to components and shards like any prop.

The counter, unchanged in behavior, no `signal` statement:

```rust
use topcoat::{Result, view::*, runtime::signal};

#[component]
async fn counter() -> Result {
    let count = signal(|| 0.0);

    view! {
        <button @click=$(|_e| count.increment())>"+1"</button>
        <p>"Count: " $(count.get())</p>
    }
}
```

Everything here is client-side exactly as today: the handler and the text expression compile to JavaScript, updates never touch the server, and the page is never refetched.

## Server reads

The new capability: `.get()` in ordinary Rust, outside any `$(...)`, reads the signal during the server render and registers it as a dependency of the page.

```rust
use topcoat::{Result, context::Cx, view::*, runtime::{signal, Event}};

#[component]
async fn product_list(cx: &Cx) -> Result {
    let query = signal(String::new);

    let products = search_products(cx, &query.get()).await?;

    view! {
        <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

        boundary(
            for product in products {
                product_card(key: &product.slug, product: product)
            }
        )
    }
}
```

The search hits the database, the loop is plain Rust over plain data, and nothing was extracted into a separate endpoint. When `query` changes in the browser, the client refetches the page. The server render is an ordinary render of ordinary code; the `signal` call resumes from the client's value instead of running `String::new`, so `search_products` sees the new query. The boundary diff from the streaming SSR design then does its job: the input, the shell, and the layouts hash identically and never travel; only the product grid comes down and is swapped in.

Reads inside `$(...)` do not register a server dependency. They are the client-reactive path and are handled entirely by the compiled JavaScript, as today. Only reads in plain Rust mark the page as depending on the signal.

## The refetch

The client knows, from metadata emitted with the page, which signals the render depended on. When one of them changes:

1. The client requests the same URL again, sending all current signal values (`X-Topcoat-State`, or a POST body; undecided) and its boundary tree hashes (`X-Topcoat-Boundaries`, as designed for client-side navigation in the streaming SSR proposal).
2. The request goes through the full route: layouts, guards, everything. This is a plain page render.
3. Every `signal` call whose identity has a value in the request resumes from it; the rest run their closures.
4. The response carries only the boundaries whose hash differs from what the client holds. It can stream: refetch composes with `defer`, so a slow region can come down as a skeleton and fill in.

Changes in the same tick coalesce into one refetch, and starting a refetch aborts one still in flight, as shard requests do today. Debounce beyond that is an open question.

Signal values round-trip through the client, so `T` needs serde in both directions. A signal used in runtime expressions additionally needs the `expr!` vocabulary, as today.

## State lifecycle

Because every render resumes signals from the client's values, state survives re-renders by construction. A `signal` call site that is not reached on a re-render, because its component unmounted, loses its state; the next time it is reached the closure runs again. Writes are client-only for now: `set` and friends exist in runtime expressions, and the server observes values but does not change them.

## Shards as an upper bound

Refetching the page for every keystroke is correct but re-renders everything to discover that one region changed. `#[shard]` becomes the opt-out: a declaration that a region is self-contained, so a signal change inside it re-renders only the shard.

A signal read in plain Rust inside a shard body registers to the shard, not the page:

```rust
use topcoat::{Result, context::Cx, view::*, runtime::{signal, shard, Signal, Event}};

#[shard]
async fn search_results(cx: &Cx, query: Signal<String>) -> Result {
    let products = search_products(cx, &query.get()).await?;
    view! {
        for product in products {
            product_card(key: &product.slug, product: product)
        }
    }
}

#[component]
async fn search(cx: &Cx) -> Result {
    let query = signal(String::new);

    view! {
        <input :value=$(query.get()) @input=$(|e: Event| query.set(e.target.value))>

        search_results(query: query)
    }
}
```

When `query` changes, the client calls the shard's endpoint instead of refetching the page, sending the shard's identity and the signal values its body read. The identity lets `signal` and `defer` call sites inside the shard hash the same as during the inline render, so the standalone re-render is indistinguishable from the region of a full page render.

The rules:

- A signal whose server reads all sit inside shards re-renders only those shards.
- A signal with any server read outside a shard refetches the page, which re-renders the shards inline and subsumes them.
- Runtime-expression arguments keep working as today: `search_results(query: $(query.get()))` with a `String` parameter still re-requests when the expression's signals change. Passing the handle is the new spelling; both trigger the same re-render.
- Signals created inside a shard now survive its re-renders, because they resume from the client's values. Today a re-render resets them; that footgun is gone.

The guard caveat from today's shards is halved, not gone. A page refetch runs layouts and guards, so the whole-page path is strictly safer than shards are now. The shard endpoint still runs the shard function alone, so a shard rendering private content still resolves authorization itself.

## Trust

Every signal with a server read is an input to an exposed endpoint. The client sends the values; anyone can send any values. This was already true of shard and procedure arguments and is now true of any signal the render reads: treat signal values like API input, always. The docs must carry this as prominently as the shard and procedure docs do. We should discuss mechanisms to make this safer as well.

## What this removes

- The `signal name = value;` statement in `view!`. One way to create a signal, usable in any component body.
- The positional declaration comment in the HTML. Signals are collected per request and emitted centrally, together with the dependency metadata the client needs. A signal created but never read anywhere is detectable, which enables a dev-mode warning.
- Shard state resets. See above.
- The pressure to restructure code around shards. The default is writing the page naturally and letting the boundary diff find what changed; a shard is a targeted efficiency claim added afterwards, in the same file, without moving code.

## Open questions

- **`cx`.** Whether `signal()` reads the request store ambiently, like `Identity::current`, or takes `cx` explicitly. Ambient matches the identity system; explicit matches everything else in Topcoat.
- **Transport.** Header versus POST body for the refetch. Headers are cute but cap out around 8-16KB at common proxies, and signal values can be large.
- **Debounce.** Same-tick coalescing plus abort-in-flight is the baseline; whether a keystroke-driven page refetch needs a real debounce knob, and where it lives.
- **Server writes.** Resetting a form after a procedure succeeds wants the server to write a signal and echo it down with the swap. Deliberately out of scope for the first draft.
- **Naming.** `signal` is kept here, but the function is general enough that `state` or `handle` may fit better, especially if non-reactive resumable state ever wants the same mechanism.
- **Untracked reads.** Whether a server read that should not register a dependency (a `peek`) is needed.
- **Change detection.** What counts as a change worth a refetch: any `set`, or only a `set` that alters the value. Value comparison on the client is cheap and avoids no-op refetches.
