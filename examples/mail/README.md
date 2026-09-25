# Mail

A welcome email declared with `mail!`, with HTML and plain-text bodies and attached files.

Run it with:

```sh
cargo topcoat dev -p mail
```

The example uses the file transport, so every message is written to `outbox/` instead of being delivered.
