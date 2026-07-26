---
name: style
description: Always use this skill before writing or editing Rust code or documentation in the Topcoat repository
---

# Topcoat Style

## Layout

Keep related code together: a struct is immediately followed by its inherent `impl` and then its trait impls, before the next struct in the file. Unit tests (`#[cfg(test)] mod tests`) go at the very bottom of the file.

Free functions are allowed, but first consider whether a more idiomatic Rust grouping onto a struct exists.

## Barrel files

Name a module's file after the module and place it alongside its directory (`foo.rs` next to `foo/`), never `foo/mod.rs`. A barrel file declares all submodules and re-exports each with a glob; only third-party items are re-exported by name.

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

Declare every dependency in the top-level `Cargo.toml` under `[workspace.dependencies]` with only a version and no features. Crates pull it in with `workspace = true` and opt into features there.

```toml
# Cargo.toml
[workspace.dependencies]
serde = "1"

# crates/topcoat/Cargo.toml
[dependencies]
serde = { workspace = true, features = ["derive"] }
```

## Procedural macros

In `Parse` impls, parse directly into the `Self { ... }` fields (`Self { x: input.parse()? }`) rather than through `let` bindings. Use a `let` only when a parsed value must be inspected to decide how to parse a later field.

## Documentation

Item docs describe what something is/does and how to use it. Avoid implementation details unless relevant to a caller. Describe the current state only; never reference previous iterations ("this used to be A but is now B").

The `docs/` folder is kept in sync with the module documentation in `crates/topcoat/src/*.rs`. Rust module docs use relative code links; the markdown docs use absolute links.

## Characters

Use only characters found on a US layout keyboard, in both code and documentation:

- `-` or `--` instead of an em dash, but preferrably avoid this em dashes entirely
- `->` instead of a Unicode arrow
- `...` instead of an ellipsis character
