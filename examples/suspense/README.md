# Suspense

Streams page sections in behind fallbacks with the `suspense` and `error_boundary` components: suspense swaps a fallback for the content when ready, an error boundary turns a failed section into a message while the section next to it renders, and a redirect thrown mid-stream turns into a client-side navigation.

Run it with:

```sh
cargo topcoat dev -p suspense
```
