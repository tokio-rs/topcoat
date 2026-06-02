# Proc Macro Style Guide

This guide captures the conventions used across `topcoat-view::ast` and
`topcoat-macro` for writing `syn`-based proc macros. New macros should match
this style so they remain readable and composable with the existing ones.

It is descriptive, not prescriptive: when in doubt, mimic the existing modules
(`component`, `shard`, `view`, `memoize`, …).

---

## 1. Module layout

### 1.1 One node = one file

Every AST node lives in its own file under the parent macro's directory. The
file is named after the type in `snake_case`:

```
crates/topcoat-view/src/ast/view/
  ├── element.rs              // pub enum Element
  ├── element_name.rs         // pub enum ElementName
  ├── element_tag.rs          // pub struct OpeningTag, ClosingTag
  ├── attribute.rs            // pub struct Attribute, BindAttribute, EventAttribute
  ├── attribute_key.rs        // pub enum AttributeKey
  ├── attribute_value.rs      // pub enum AttributeValue
  ├── template_if.rs          // pub struct TemplateIf<T>, TemplateElse<T>
  ├── template_for_loop.rs    // pub struct TemplateForLoop<T>, …
  ├── view_writer.rs          // pub(crate) struct ViewWriter
  └── mod.rs
```

Tightly-coupled siblings (e.g. `OpeningTag` + `ClosingTag`, `TemplateIf` +
`TemplateElse`) may share a file. Otherwise split.

### 1.2 `mod.rs` is a flat re-export hub

Submodules are private; their `pub` items are re-exported from `mod.rs`:

```rust
// crates/topcoat-view/src/ast/view/mod.rs
mod attribute;
mod attribute_key;
// …

pub use attribute::*;
pub use attribute_key::*;
// …
pub(crate) use view_writer::*;   // helpers stay crate-private
```

The top-level type (`View`, `Component`, `Shard`, …) is defined directly in
`mod.rs`. It owns the `Parse` and `ToTokens` impls that delegate into the
writer/builder.

### 1.3 Attribute macros: the three-part split

Every `#[proc_macro_attribute]` macro decomposes into exactly three pieces in
its own directory:

```
component/
  ├── attr.rs   // pub struct ComponentAttr;  impl Parse for ComponentAttr
  ├── item.rs   // pub struct ComponentItem;  impl Parse with validation
  └── mod.rs    // pub struct Component { _attr, item }
                // impl Component { new, parse }
                // impl ToTokens for Component
```

- `XxxAttr` parses the `#[xxx(...)]` arguments. Empty? Still write the type
  and an empty `Parse` impl — keeps the shape consistent and makes adding
  options later a non-breaking change. See `component/attr.rs:3-9`.
- `XxxItem` parses the annotated item and **does all up-front validation**.
- `Xxx` wraps both, exposes `new` + `parse`, and `impl ToTokens for Xxx` does
  the actual codegen.

The `parse` associated function is the public entry point used by `lib.rs`:

```rust
impl Component {
    pub fn new(attr: ComponentAttr, item: ComponentItem) -> Self {
        Self { _attr: attr, item }
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}
```

Prefix unused fields with `_` (`_attr`) rather than removing them — that
documents the intent and reserves the slot for future options.

### 1.4 Function-like macros

`#[proc_macro] view!(...)` and `#[proc_macro] segment!(...)` don't need the
attr/item split. They expose a single top-level type with `Parse` + `ToTokens`,
and `lib.rs` uses `syn::parse_macro_input!` directly.

---

## 2. `lib.rs` is glue, not logic

`crates/topcoat-macro/src/lib.rs` only:

1. Declares the proc-macro entry points behind `#[cfg(feature = "...")]`.
2. Delegates to the AST type's `parse` (or `syn::parse_macro_input!`).
3. Converts the result via `quote! { #value }.into()` on success, or
   `error.to_compile_error().into()` on failure.

The standard attribute-macro entry point is one block:

```rust
#[cfg(feature = "view")]
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::component::Component::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
```

No business logic. No string building. No conditional `quote!`. If you find
yourself reaching for `if` or `match` in `lib.rs`, the logic belongs in the
AST type.

User-facing doc comments (`///`) for the macro itself live in `lib.rs`, above
the proc-macro fn. The AST file gets implementation-doc comments (what the
parsed shape represents, invariants, span behavior).

---

## 3. Parsing

### 3.1 The `ParseOption` trait

`crates/topcoat-view/src/ast/parse_option.rs` defines:

```rust
pub trait ParseOption: Parse + Sized {
    fn peek(input: ParseStream) -> bool;

    fn parse_option(input: ParseStream) -> syn::Result<Option<Self>> {
        Self::peek(input).then(|| input.parse()).transpose()
    }
}
```

Every AST node that can appear at a *position* (not just inside a fixed
sequence) implements `ParseOption`. This is the project's lookahead protocol:

```rust
impl ParseOption for Element {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![<])
    }
}

impl ParseOption for TemplateIf<T> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![if])
    }
}
```

**Rule:** if a node can appear inside an `else if`-style dispatch in another
node's `Parse` impl, it must implement `ParseOption`.

### 3.2 Enum dispatch in `Parse`

Choice points are written as a flat `if … else if …` chain over
`Foo::peek(input)` calls — never as `match` on `input.parse::<TokenStream>()`,
never speculatively forking unless absolutely necessary:

```rust
impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = if input.peek(LitStr) {
            Self::Text(input.parse()?)
        } else if DocumentType::peek(input) {
            Self::DocumentType(input.parse()?)
        } else if Element::peek(input) {
            Self::Element(input.parse()?)
        }
        // … one arm per variant …
        else {
            return Err(syn::Error::new(input.span(), "expected view node"));
        };

        // Post-parse rejection of variants we know about but don't support yet.
        match result {
            Self::Continue(inner) => Err(syn::Error::new(
                inner.expr_continue.span(),
                "`continue` is currently not supported",
            )),
            // …
            _ => Ok(result),
        }
    }
}
```

Notes:

- The dispatch order can matter — put more specific peeks before more general
  ones (e.g. `DocumentType::peek` before `Element::peek`, since both start
  with `<`).
- For two-way ambiguity prefer `lookahead1()`/`lookahead.peek(...)` so the
  error message lists every alternative automatically (`element_name.rs:95`,
  `attribute_value.rs:30`).
- Use `input.fork()` only when no amount of peeking can decide: `component.rs`
  forks to test for a `Path` followed by `(`.

### 3.3 Validation belongs in `Parse`

When the shape of a parsed item must satisfy invariants, enforce them in
`Parse` (typically in the `XxxItem`'s impl). Downstream code — including
`ToTokens` — may then rely on those invariants:

```rust
// component/item.rs — validates async, return type, no `self`, ident-only pats.
impl Parse for ComponentItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "components must be async",
            ));
        }
        // … further checks …
        Ok(Self { item })
    }
}

// component/mod.rs — relies on those checks.
let FnArg::Typed(pat_type) = input else {
    unreachable!("validated in Parse");
};
let Pat::Ident(pi) = &*pat_type.pat else {
    unreachable!("validated in Parse");
};
```

Always tag `unreachable!` calls with `"validated in Parse"` so the contract is
visible. Prefer `let … else { unreachable!(...) }` over a `match` with a
single arm and a fallthrough.

### 3.4 Error construction

- `syn::Error::new_spanned(node, msg)` when you have an AST node — gives the
  best span.
- `syn::Error::new(span, msg)` when you only have a `Span` (e.g.
  `input.span()`, a token's `.span()`).
- Error messages are lowercase, no trailing period, and use backticks for
  identifiers/tokens: `"`continue` is currently not supported"`,
  `"missing closing tag for opening tag `{name}`"`.

### 3.5 Sub-parsers and `input.call`

Use `input.call(...)` for one-off parsers from `syn`:

- `input.call(Expr::parse_without_eager_brace)?` — the standard way to parse a
  condition or iterator expression that's followed by a `{`-delimited body.
- `input.call(Pat::parse_single)?` / `Pat::parse_multi_with_leading_vert(input)?`
  for patterns.
- `input.call(MyType::parse_option)?` to chain optional sub-parses inside a
  `while let Some(...)` loop (see `attributes.rs:30`).

### 3.6 Delimited groups

For `(...)`, `{...}`, `[...]`:

```rust
let content;
Ok(Self {
    paren: parenthesized!(content in input),  // or braced!, bracketed!
    expr: content.parse()?,
})
```

The token (`Paren`, `Brace`) is **stored on the struct** even when it isn't
used at codegen time — its span is needed for diagnostics and for the
pretty-printer (`template_block.rs:47-65`).

### 3.7 Custom keywords

When a node starts with a contextual keyword (not a real Rust keyword), use
`syn::custom_keyword!` in a local `mod kw`:

```rust
mod kw {
    use syn::custom_keyword;

    custom_keyword!(signal);
    custom_keyword!(DOCTYPE);
    custom_keyword!(track);
}

pub struct SignalDeclaration {
    pub signal_kw: kw::signal,
    // …
}

impl ParseOption for SignalDeclaration {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::signal)
    }
}
```

One `mod kw` per file. Don't share `kw` across files — each node owns its
sigils.

### 3.8 Generic-over-body nodes

When the same control-flow construct appears in multiple positions (view body
vs. attribute list), make it generic over the body type:

```rust
pub struct TemplateIf<T> {
    pub if_token: Token![if],
    pub cond: syn::Expr,
    pub then_branch: TemplateBlock<T>,
    pub else_branch: Option<TemplateElse<T>>,
}

impl<T: Parse> Parse for TemplateIf<T> { … }
impl<T: WriteView> WriteView for TemplateIf<T> { … }
impl<T: Parse> ParseOption for TemplateIf<T> { … }
```

Consumers instantiate with `TemplateIf<Nodes>` (view body) or
`TemplateIf<AttributeNodes>` (attribute list).

---

## 4. Codegen

### 4.1 Prefer a writer/builder to a single `quote!`

When generated code requires a non-trivial composition of statements (multiple
chunks, control flow, optimization passes), introduce a `crate`-private writer
type with a `Write...` trait that AST nodes implement.

The model is `ViewWriter` + `WriteView` in `ast/view/view_writer.rs`:

- AST nodes don't implement `ToTokens` directly. They implement
  `WriteView::write(&self, writer: &mut ViewWriter)`.
- The writer accumulates *chunks* (literal text, expression, `let`, `if`,
  `for`, `match`, raw statement) and flushes adjacent literal text into one
  `Unescaped::new_unchecked("…")` call.
- The top-level `impl ToTokens for View` constructs a `ViewWriter`, walks the
  tree, and calls `writer.into_token_stream()`.
- `into_token_stream` chooses the cheapest output shape based on what's in the
  writer (empty / single expr / iter-chain / imperative `Vec`).

Use this pattern whenever:

- Output has more than one statement *and* a structure that benefits from
  peephole optimization or escape analysis.
- The same construct produces different code depending on what surrounds it
  (e.g. expression context vs. statement context).

Don't reach for it when a single `quote! { ... }` would do. The component and
shard macros are simple enough to skip the writer.

### 4.2 `ToTokens` shape

Inside `impl ToTokens for Foo`, build one `quote! { ... }` and pipe it to
`tokens` at the end:

```rust
impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // … local computation …
        quote! {
            #[allow(non_camel_case_types)]
            #vis struct #ident #impl_generics #where_clause { … }

            impl #impl_generics ::topcoat::view::Component for #ident #ty_generics #where_clause { … }
        }
        .to_tokens(tokens);
    }
}
```

Avoid returning a `TokenStream` and assigning it — emit straight into the
caller's `tokens`. Multiple `quote! { ... }.to_tokens(tokens);` calls in
sequence are fine (see `shard/mod.rs` for the `cfg!(feature = "discover")`
trailing block).

### 4.3 Fully-qualified absolute paths

Every reference to a runtime item is written with a leading `::`:

- `::topcoat::view::Component`
- `::topcoat::runtime::ReactiveScope`
- `::topcoat::context::Cx`
- `::core::iter::IntoIterator`, `::core::option::Option`
- `::std::vec::Vec`

Never use bare `Vec`, `Option`, `IntoIterator`, etc. in generated code — the
user's crate may have shadowed them.

### 4.4 Hygiene-prefixed identifiers

Any identifier the macro *introduces* into the user's scope uses a double
underscore prefix:

- `__cx`, `__implicit`, `__v` — local bindings
- `__element_name_{n}` — uniquified via an `AtomicU32` counter
- `'__cx`, `'__implicit` — lifetimes

When uniqueness across expansions is required (e.g. multiple element-name
expressions in one view), generate fresh names with a `static AtomicU32`
counter:

```rust
static AUTO_INCREMENT: std::sync::atomic::AtomicU32 = …::new(0);
let increment = AUTO_INCREMENT.fetch_add(1, Ordering::Relaxed);
let name_ident = Ident::new(&format!("__element_name_{}", increment), Span::call_site());
```

### 4.5 `parse_quote` for synthetic AST nodes

When you need a `syn` AST node from a literal string (e.g. to splice into
`generics.params`), use `parse_quote!`:

```rust
generics.params.insert(0, parse_quote! { '__cx });
item.sig.inputs.insert(0, parse_quote! { __cx: &'__cx ::topcoat::context::Cx });
```

This keeps the synthetic and user-written AST homogeneous and avoids
hand-building `syn::GenericParam`/`syn::FnArg`.

### 4.6 `VisitMut` for AST transforms

When you need to rewrite parts of a user-written `Type` / `Expr` / `Pat`, use
`syn::visit_mut::VisitMut`:

```rust
struct ImplicitLifetimeVisitor { used: bool }

impl VisitMut for ImplicitLifetimeVisitor {
    fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
        if lt.ident == "_" {
            *lt = parse_quote! { '__implicit };
            self.used = true;
        }
    }

    fn visit_type_reference_mut(&mut self, tr: &mut TypeReference) {
        if tr.lifetime.is_none() {
            tr.lifetime = Some(parse_quote! { '__implicit });
            self.used = true;
        }
        visit_mut::visit_type_reference_mut(self, tr);  // recurse!
    }
}
```

Remember to delegate to the default walker (`visit_mut::visit_X_mut(self, x)`)
inside non-leaf overrides, otherwise the rest of the subtree is skipped.

### 4.7 `QuoteOption` for round-tripping `Option`

`quote::ToTokens` on `Option<T>` emits nothing for `None` and the inner value
for `Some`. When generated code should still contain an `Option`, wrap with
`crate::quote_option::QuoteOption` so it round-trips as
`::core::option::Option::Some(...)` / `::core::option::Option::None`.

---

## 5. Tests

### 5.1 Layout

Tests live at the bottom of the AST file under `#[cfg(test)] mod tests`. Two
helpers are standard:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Foo {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Foo>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    // …
}
```

Use `syn::parse_str` (not `syn::parse2`) for tests — string input is what's
readable, and it round-trips through the proc-macro2 fallback so spans work.

### 5.2 Test names

Test names read as a present-tense statement of the behavior under test:

- `parses_normal_element`
- `parses_void_element_without_closing_tag`
- `missing_closing_tag_is_rejected`
- `mismatched_closing_tag_is_rejected`
- `dispatches_each_variant`
- `is_void_element_only_matches_known_void_idents`

For error-case tests, use the `_is_rejected` suffix and check the message
substring:

```rust
#[test]
fn missing_closing_tag_is_rejected() {
    assert!(parse_err("<div>").contains("missing closing tag"));
}
```

### 5.3 What to cover

For each AST node, at minimum:

1. One test per syntactically-distinct shape (e.g. void vs. normal element,
   each variant of an enum).
2. One test per validation rule, asserting the substring of the error.
3. One test for every behavioral method (`is_void_element`, `string_name`,
   `expr`, …).

The codegen output itself is **not** unit-tested at this layer — integration
and runtime tests in other crates cover that.

---

## 6. Pretty printing

The `feature = "pretty"` flag pulls in `topcoat_pretty::PrettyPrint`. Every AST
node gets a parallel impl, gated by the feature, immediately after its
`Parse`/`WriteView`/`ToTokens` impls:

```rust
#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Element { … }
```

Don't fold pretty-printing into the codegen impls — keep it separate so the
non-pretty build path stays minimal. New AST nodes that don't yet need
printing may use `todo!()` (see `reactive_scope.rs`, `signal_declaration.rs`),
but a real impl is expected before the node graduates beyond experimental.

---

## 7. Documentation

### 7.1 Doc on the AST type, not the impl

Each `pub` AST type carries a `///` comment describing **what construct it
represents in the source language**, not what it does in code:

```rust
/// An HTML element. `Void` covers the HTML void elements (`<br>`, `<img>`, …)
/// which take no closing tag and no children.
pub enum Element { … }

/// A `for pat in expr { ... }` loop in view-body position. The body is
/// rendered once per iteration.
pub struct TemplateForLoop<T> { … }
```

The first sentence is the one-liner that shows up in summaries. If a type is
generic-over-body, the doc names which position it serves.

### 7.2 Inline rustdoc on `lib.rs`

User-facing rustdoc — the doc the *consumer of the proc macro* reads — lives
above the `#[proc_macro_attribute] fn …` in `lib.rs`. Include:

- A one-line summary of what the attribute does.
- An `# Examples` section using ```rust ignore``` fences.
- A `# Requirements` section listing trait bounds and shape constraints.

`path_param`, `query_params`, and `memoize` in
`crates/topcoat-macro/src/lib.rs` are the reference for that format.

### 7.3 Comments inside macro code

Sparse. The standards are:

- Above a non-obvious heuristic explaining *why* it's safe or correct.
  Example: the `peek_named_arg` helper in `view/component.rs:64-68`
  documenting that `::` would start a path.
- Above codegen branches noting which optimized path they're taking
  (`view_writer.rs:127, 134, 167`).
- Above `unreachable!` calls when the relied-on invariant isn't obvious from
  context.

Don't restate what the next line of code says.

---

## 8. Quick checklist for a new macro

When adding a new attribute macro `#[foo]`:

1. **Crate wiring**
   - Add `pub fn foo(attr, item)` in `topcoat-macro/src/lib.rs` behind any
     relevant `#[cfg(feature = "...")]`. Body is the standard `match` →
     `quote!`/`to_compile_error` glue.
   - Add user-facing rustdoc to that fn (summary + `# Examples` +
     `# Requirements`).

2. **AST module**
   - Create `topcoat-macro/src/foo/` (or the appropriate sibling crate) with
     `attr.rs`, `item.rs`, `mod.rs`.
   - `FooAttr` parses (even if empty); `FooItem` parses *and validates*; `Foo`
     wraps both with `new` + `parse` + `ToTokens`.

3. **Parsing**
   - Validate everything you intend to rely on. Use `Error::new_spanned` where
     a node is available.
   - If the macro participates in a position-based grammar, implement
     `ParseOption` and follow the peek-then-parse dispatch idiom.

4. **Codegen**
   - Single `quote! { ... }.to_tokens(tokens)` for simple cases.
   - Introduce a writer + `Write...` trait when output composition is
     non-trivial.
   - Fully-qualify every external path (`::topcoat::…`, `::core::…`,
     `::std::…`).
   - Prefix every introduced ident/lifetime with `__`.

5. **Tests**
   - `parse` + `parse_err` helpers at the bottom of each AST file.
   - One test per shape, one per validation rule, one per public method.

6. **Pretty printing**
   - Add a `#[cfg(feature = "pretty")] impl PrettyPrint` next to each Parse
     impl. `todo!()` is acceptable for experimental nodes; spell out the
     real intent in a doc comment.

If everything in this checklist is in place, the macro will read like the rest
of the crate.
