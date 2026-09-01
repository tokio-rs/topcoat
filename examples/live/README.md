# Live

Pages that send their shell immediately and stream slow content in with `live!` and `emit!`: one page swaps a loading message for the data once it arrives, one reports progress step by step, and one catches a failed emission and swaps in a fallback instead.

Run it with:

```sh
cargo topcoat dev -p live
```
