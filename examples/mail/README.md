# Mail

A welcome mail built with the `mail!` macro: an HTML body with a derived plain-text alternative, an inline image, an attachment, and a custom header.

Run it with:

```sh
cargo topcoat dev -p mail
```

The example uses the file transport, so every message is written to `outbox/` instead of being delivered.
