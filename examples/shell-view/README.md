# Shell view

This example streams a dashboard shell with two placeholders, then replaces each placeholder when its component finishes.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/shell-view/Cargo.toml
```

Open `http://127.0.0.1:3000/`. The shell appears first, recent activity appears after one second, and recommendations appear after two seconds.
