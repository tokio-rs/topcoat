# Shard

A search box that passes its query signal to a `#[shard]`. The server renders new results when the query changes. A local result limit shows how a shard preserves its own state across renders.

Run it with:

```sh
cargo topcoat dev -p shard
```
