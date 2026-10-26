# Context

This example reads shared and local values through `Cx`. The router gives every request the same page view counter and a default greeting. A layer uses `cx.with` to change the greeting for `/formal`. A plain function reads the current request path.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p context
```
