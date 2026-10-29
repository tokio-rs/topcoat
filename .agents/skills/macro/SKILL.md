---
name: macro
description: Always use this skill before writing a macro for Topcoat or changing a macro's syntax, including `macro_rules!` macros
---

## Syntax

Every macro follows the same argument style, so users can guess the syntax of a macro they have not seen yet.

* Attribute macros take options as `key = value`, like `#[serde(rename = "...")]`. Flags are bare words, and nested values use call syntax: `#[query_params(error = bad_request)]`, `#[memoize(as_ref)]`, `#[page("/about", with(Title("About")))]`.
* Function-like macros take named options as `key: value`, the same as component props in `view!`: `asset!("./logo.png", rename: "logo")`, `t!(greeting, name: user.name)`. The value is a plain Rust expression without parentheses. A bare key is shorthand for a variable of the same name, like field init shorthand in a struct literal.
* A macro that copies a syntax people already know follows that syntax as closely as Rust's tokenizer allows: `view!` copies HTML, `font!` copies CSS `@font-face`, and `cookie!` copies the `Set-Cookie` header. Inside it, a literal that the copied syntax would contain is written as it is (`class="card"`, `Max-Age = 3600`, `SameSite = Lax`), and any Rust expression is wrapped in `(...)` (`hidden=(collapsed)`, `session = (token)`). Do not wrap literals in parentheses, since the macro can only handle a literal at compile time while it can see that it is one.
* A visibility comes first: `messages!(pub "...")`, `path_param!(pub post_id: u64)`.
* The main input comes first and has no name, such as the asset path, the message ID, or the link target.
* A choice from a fixed set is a bare Pascal case name: `kind: Static`, `style: Normal`, `GET`.
* An error response names a router error constructor, with or without arguments: `not_found`, `bad_request("invalid id")`. Where `:` already means "has type", attach it with `else` instead of an option: `path_param!(pub post_id: u64 else not_found)`.

## AST nodes

* Like `syn` nodes, an AST node is a lossless record of the syntax it matched: every field is `pub` and holds the tokens as written, spans included. The original source should be reconstructible from the node. Field order does not have to match the source, but spans must be preserved.
* Parsing validates, it does not interpret. A `Parse` impl rejects what is not valid syntax and stores everything else verbatim in token fields; it never normalizes, resolves, or drops information on the way in.
* Semantics are derived from the AST, not baked into it. Put the interpretation in methods on the node that read its own fields, so the node stays a faithful representation of the source and the meaning stays separate from it.

## Parsing

* In `Parse` impls, parse directly into the `Self { ... }` fields (`Self { x: input.parse()? }`) rather than through `let` bindings. Use a `let` only when a parsed value must be inspected to decide how to parse a later field.
* To parse custom keywords that are not re-emitted into the generated code, create a private `mod kw` with `syn::custom_keyword!` invocations instead of parsing `syn::Ident`. Use `input.lookahead1()` if it makes sense.
* Instead of manually peeking for optional AST nodes in the input stream at the call site, implement `ParseOption` and parse them with `input.call(Node::parse_option)?`.

## Entry points

The `macro/` crate only bridges `proc_macro::TokenStream` to the grammar crate that holds the AST and the codegen. Every entry point is a parse, a match, and a `to_compile_error()` on the error arm; no logic lives in `lib.rs`.

An attribute macro receives two token streams, so it gets three types: one `Parse` node per stream and a tuple struct pairing them, which is the node the codegen hangs off.

```rust
// grammar crate
pub struct ProcedureAttr {}
pub struct ProcedureItem { ... }

pub struct Procedure(ProcedureAttr, ProcedureItem);

impl Procedure {
    #[must_use]
    pub fn new(attr: ProcedureAttr, item: ProcedureItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Procedure { ... }
```

`Parse` stays per stream, since neither stream alone is the macro input. The pairing type owns the inherent `parse` that joins them and the `ToTokens` impl that expands them, which keeps the entry point a one-liner:

```rust
// macro crate
#[doc = include_str!("../docs/procedure.md")]
#[proc_macro_attribute]
pub fn procedure(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_runtime_grammar::procedure::Procedure::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
```

A function-like macro has a single input, so it needs no pairing type: `syn::parse_macro_input!` straight into its AST node, then expand.
