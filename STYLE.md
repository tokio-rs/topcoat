# Topcoat Style Guide

## Locality of behavior

Strive to keep related code together. A struct is immediately followed by its
inherent `impl` block and then its trait impls, before the next struct is
declared in the file.

## Barrel files

A module's barrel file declares all submodules and then re-exports each with a
glob (`pub use submodule::*;`) rather than re-exporting individual items. The
exception is third-party items, which are re-exported by name for convenience
(e.g. `pub use http::Method;`).

Name a module's file after the module itself, placed alongside its directory
(`foo.rs` next to `foo/`), rather than using `foo/mod.rs`.

```rust
mod content;
mod error;
mod request;

pub use content::*;
pub use error::*;
pub use request::*;

pub use http::Method;
```

## Dependencies

Declare every dependency in the top-level `Cargo.toml` under
`[workspace.dependencies]` with only its version and no features. Individual crates
pull it in with `workspace = true` and opt into the features they need there.

```toml
# Cargo.toml
[workspace.dependencies]
serde = "1"

# crates/topcoat/Cargo.toml
[dependencies]
serde = { workspace = true, features = ["derive"] }
```

## Procedural macros

### Parse into the struct constructor

In `Parse` impls, parse directly into the `Self { ... }` fields
(`Self { x: input.parse()? }`) rather than through `let` bindings. Only use a
`let` when a parsed value must be inspected to decide how to parse a later field.

## Documentation

Item docs describe what something is/does and how to use it. Avoid mentioning
implementation details unless they're relevant to a caller. Never reference
previous iterations of the code (e.g. "this used to be A but is now B").
Documentation describe the current state only.

The `docs/` folder contains markdown documentation which is kept in sync with
module documentation in `crates/topcoat/src/*.rs`. However, the Rust module documentation
should use relative code links whereas the markdown documentation uses absolute links.

## Characters

Write documentation and code using only characters found on a US layout
keyboard. Avoid special characters such as em dashes, Unicode arrows, or the
ellipsis. Use their plain-ASCII equivalents instead:

- `-` or `--` instead of an em dash (`—`)
- `->` instead of a Unicode arrow (`→`)
- `...` (three periods) instead of a single ellipsis character (`…`)

Use em dashes sparingly.

## Tests

Unit tests (`#[cfg(test)] mod tests`) always go at the very bottom of the file.

