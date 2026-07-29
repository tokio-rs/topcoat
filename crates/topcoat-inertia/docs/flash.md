# Flash storage

Flash data crosses a redirect once. [`flash`](crate::flash) adds page-level data, while [`flash_errors`](crate::flash_errors) adds the reserved validation object. History flags set by [`clear_history_on_redirect`](crate::clear_history_on_redirect) and [`preserve_fragment_on_redirect`](crate::preserve_fragment_on_redirect) use the same lifecycle.

The Inertia layer reads the store at the start of a request. Redirects preserve incoming and newly added data. A rendered page includes the incoming data and deletes it after the response is prepared. Ordinary responses that do not render an Inertia page do not consume it.

## Private cookie store

[`CookieFlashStore`](crate::CookieFlashStore) is the default. It encrypts and authenticates the payload through Topcoat's private cookie jar and requires both `.cookies()` and a persistent [`Key`](topcoat_cookie::Key) registered as app context.

The key must survive restarts and must be shared by every server process or serverless isolate. The store contains no process-local fallback. Invalid or tampered private cookies are treated as absent by the cookie jar.

The cookie is HTTP-only, same-site lax, secure by default, scoped to `/`, and short-lived. Set `secure(false)` only for local plain-HTTP development.

## Size and custom stores

The default rejects a projected cookie larger than its safe limit. Cookie encryption and response attributes add overhead, so the raw JSON limit is smaller than the browser's nominal cookie limit.

Implement [`FlashStore`](crate::FlashStore) when flash or validation payloads can be large. A custom store receives an opaque byte payload and the current request context. Store it by a session identifier, not in a process-local map, when requests can move between processes or isolates.

```rust
use topcoat_core::context::Cx;
use topcoat_inertia::{FlashStore, FlashStoreFuture};

struct Store;

impl FlashStore for Store {
    fn read<'a>(&'a self, _cx: &'a Cx) -> FlashStoreFuture<'a, Option<Vec<u8>>> {
        Box::pin(async { Ok(None) })
    }

    fn write<'a>(&'a self, _cx: &'a Cx, _payload: &'a [u8]) -> FlashStoreFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }

    fn delete<'a>(&'a self, _cx: &'a Cx) -> FlashStoreFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
```

In production, propagate storage failures so the redirect does not silently lose validation state.
