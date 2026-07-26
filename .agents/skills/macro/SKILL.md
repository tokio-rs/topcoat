---
name: macro
description: Always use this skill before writing procedural macros for Topcoat
---

## AST nodes

* Like `syn` nodes, an AST node is a lossless record of the syntax it matched: every field is `pub` and holds the tokens as written, spans included. The original source should be reconstructible from the node. Field order does not have to match the source, but spans must be preserved.
* Parsing validates, it does not interpret. A `Parse` impl rejects what is not valid syntax and stores everything else verbatim in token fields; it never normalizes, resolves, or drops information on the way in.
* Semantics are derived from the AST, not baked into it. Put the interpretation in methods on the node that read its own fields, so the node stays a faithful representation of the source and the meaning stays separate from it.

## Parsing

* In `Parse` impls, parse directly into the `Self { ... }` fields (`Self { x: input.parse()? }`) rather than through `let` bindings. Use a `let` only when a parsed value must be inspected to decide how to parse a later field.
* To parse keywords, create private `mod kw` with `syn::custom_keyword!` invocations instead of parsing `syn::Ident`.
