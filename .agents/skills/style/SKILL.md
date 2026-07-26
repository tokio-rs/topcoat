---
name: style
description: Always use this skill before writing or editing Rust code or documentation in the Topcoat repository
---

# Code Style

## General

* Keep related code together: a struct is immediately followed by its inherent `impl` and then its trait impls, before the next struct in the file. Unit tests (`#[cfg(test)] mod tests`) go at the very bottom of the file.
* Free functions are allowed, but first consider whether a more idiomatic Rust grouping onto a struct exists.
* Unsafe code is not allowed in this project, unless wrapped by a reputable dependency.

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

* Declare every dependency in the top-level `Cargo.toml` under `[workspace.dependencies]` with only a version and no features. Crates pull it in with `workspace = true` and opt into features there.

## Procedural macros

* In `Parse` impls, parse directly into the `Self { ... }` fields (`Self { x: input.parse()? }`) rather than through `let` bindings. Use a `let` only when a parsed value must be inspected to decide how to parse a later field.
* To parse keywords, create private `mod kw` with `syn::custom_keyword!` invocations instead of parsing `syn::Ident`.

## Documentation

* Item docs describe what something is/does and how to use it. Avoid implementation details unless relevant to a caller. Describe the current state only; never reference previous iterations ("this used to be A but is now B").
* Use only ASCII characters in both code and documentation, e.g. `->` instead of unicode arrow or `...` instead of ellipsis character. Avoid em-dashes entirely.
* Avoid using `ignore` for code snippets to keep them type-checked.
