---
name: style
description: Always use this skill before writing or editing Rust code or documentation in the Topcoat repository
---

# Code Style

## General

* Keep related code together: a struct is immediately followed by its inherent `impl` and then its trait impls, before the next struct in the file. Unit tests (`#[cfg(test)] mod tests`) go at the very bottom of the file.
* Free functions are allowed, but first consider whether a more idiomatic Rust grouping onto a struct exists.
* Unsafe code is not allowed in this project, unless wrapped by a reputable dependency.
* Avoid needless allocations, it is reasonable to refactor the code a bit to make it faster.

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

When a module's submodules are peers that make up a whole (the CLI commands `fmt`, `dev`, `asset`; the macros `expr`, `procedure`, `shard`), put anything shared between them in a `common` submodule so it does not read as another peer.

## Dependencies

* Declare every dependency in the top-level `Cargo.toml` under `[workspace.dependencies]` with only a version and no features. Crates pull it in with `workspace = true` and opt into features there.

## Documentation

* Write in plain english. No fancy sentence structure.
* Item docs describe what something is/does and how to use it. Avoid implementation details unless relevant to a caller.
* Describe the current state only; never reference previous iterations ("this used to be A but is now B").
* Avoid exhaustively listig specific implementations or uses that could evolve over time and go stale. Keep documentation robust to changes.
* Avoid mentioning unrelated items, like "this is used by X to do Y". Focus on describing only the item itself.
* Use only ASCII characters in both code and documentation, e.g. `->` instead of unicode arrow or `...` instead of ellipsis character.
* Avoid em-dashes entirely. Use colons and semicolons sparingly.
* Avoid using `ignore` for code snippets to keep them type-checked.

## Tests

* Do not write tests that only verify hardcoded values. Make the tests insensitive to the exact wording of strings or error messages.
* Every test must protect a specific behavior or catch a plausible bug. Before writing it, identify what incorrect behavior would make it fail.
* Derive expected results from the intended behavior. Do not calculate them by repeating the implementation or calling the same code being tested.
* Keep tests sensitive to broken behavior and tolerant of implementation changes that preserve correct behavior.
* If a test is useless, delete it.
