# Mail

A welcome email declared with `mail!`, with HTML and plain-text bodies and attached files.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p mail
```

The example uses the file transport, so every message is written to `outbox/` instead of being delivered.
