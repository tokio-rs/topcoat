# Shard

A search box whose results are searched on the server. The query is stored in a signal and passed to a `#[shard]` as an argument, so the shard renders again whenever the query changes. The shard also keeps state of its own: a "show more" limit it creates, reads on the server, and which survives its re-renders.

Run it with:

```sh
cargo topcoat dev -p shard
```
