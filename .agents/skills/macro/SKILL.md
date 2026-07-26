---
name: macro
description: Always use this skill before writing procedural macros for Topcoat
---

* Like `syn` nodes, all Topcoat AST nodes should have `pub` fields that parse the tokens including their spans.
* In `Parse` impls, parse directly into the `Self { ... }` fields (`Self { x: input.parse()? }`) rather than through `let` bindings. Use a `let` only when a parsed value must be inspected to decide how to parse a later field.
* To parse keywords, create private `mod kw` with `syn::custom_keyword!` invocations instead of parsing `syn::Ident`.
