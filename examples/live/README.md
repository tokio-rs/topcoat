# Live

Pages that send their shell immediately and stream slow content in with `live!` and `emit!`: suspense swaps a placeholder for the content once it arrives, progress reports a long-running task step by step, and error handling catches a failed emission and swaps in a fallback instead.

Run it with:

```sh
cargo topcoat dev -p live
```
