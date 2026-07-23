---
name: style
description: Always use this skill before writing or editing Rust code or documentation in the Topcoat repository
---

# Topcoat Style

Load this skill before writing or editing code or documentation in this project.

**Always read [`STYLE.md`](../../../STYLE.md) before making a change.** It is the
authoritative style guide; [`AGENTS.md`](../../../AGENTS.md) describes the
workspace layout and points to the per-feature guides under `docs/`. This skill
distills the rules that are easiest to miss.

## Safety

Only safe code. `unsafe` is not allowed.

## Locality of behavior

Keep related code together. A struct is immediately followed by its inherent
`impl` block and then its trait impls, before the next struct is declared.

## Barrel files

A module's barrel file declares its submodules and re-exports each with a glob
rather than listing individual items. Third-party items are re-exported by name.
Name a module file after the module and place it alongside its directory
(`foo.rs` next to `foo/`), never `foo/mod.rs`.

```rust
mod content;
mod error;

pub use content::*;
pub use error::*;

pub use http::Method;
```

## Dependencies

Declare every dependency once in the top-level `Cargo.toml` under
`[workspace.dependencies]` with only a version and no features. Each crate pulls
it in with `workspace = true` and opts into the features it needs.

## Procedural macros

In `Parse` impls, parse directly into the `Self { ... }` fields rather than
through `let` bindings. Use a `let` only when a parsed value must be inspected to
decide how to parse a later field.

## Documentation

Item docs say what something is and how to use it; avoid implementation detail
unless it matters to a caller. Describe the current state only -- never reference
previous iterations of the code.

The `docs/` markdown mirrors the module documentation in
`crates/topcoat/src/*.rs`. Rust module docs use relative code links; the
markdown under `docs/` uses absolute links.

## Characters

Use only plain-ASCII characters found on a US keyboard: `-` or `--` instead of
an em dash, `->` instead of a Unicode arrow, `...` instead of an ellipsis.

## Tests

Unit tests (`#[cfg(test)] mod tests`) go at the very bottom of the file. See the
[`check`](../check/SKILL.md) skill for how to run the test suite.
