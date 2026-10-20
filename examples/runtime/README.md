# Runtime

These pages show how to build interactive views with Topcoat's browser runtime:

- `counter` updates a signal in response to browser events.
- `show` binds an HTML attribute to a signal's value.
- `sort` reads a signal on the server, causing the page to render again when that signal changes.
- `procedure` calls a server function from a browser event handler.
- `shard` asks the server for new search results as the query changes.
- `mount` runs code when the page loads: it formats a time in the viewer's time zone, shows a message saved before a reload, and resumes an interrupted operation.
- `record` stores an order as a `#[record]` value in a signal and submits it to a server procedure.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then start the example with:

```sh
cargo topcoat dev -p runtime
```
