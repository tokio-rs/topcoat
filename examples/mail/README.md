# Mail

Build a welcome email with `mail!` and write it to a file for inspection.

Run it with:

```sh
cargo topcoat dev -p mail
```

The example uses the file transport, so every message is written to `outbox/` instead of being delivered.
