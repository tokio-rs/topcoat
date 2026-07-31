# Shell view

This example builds a page shell with navigation and a nested content shell. The content shell contains three deferred portlets. Activity and recommendations use `ShellViewBuilder::defer`, while the newsfeed uses inline `defer` syntax inside `shell_view!`.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/shell-view/Cargo.toml
```

Open `http://127.0.0.1:3000/`. The page and content shells appear first. Each portlet replaces its placeholder when its component finishes.
