# Context

Pages that read values through `Cx`. A page view counter and a greeting are registered on the router. A layer replaces the greeting under `/formal` with `cx.with`, and a plain function reads the request path.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p context
```
