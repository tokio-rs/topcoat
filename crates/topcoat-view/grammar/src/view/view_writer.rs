use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Expr, Ident, Pat};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

/// AST nodes that can emit themselves into a [`ViewWriter`].
pub(crate) trait WriteView {
    fn write(&self, writer: &mut ViewWriter);
}

/// Builds the `TokenStream` that a `view!` invocation expands to.
///
/// Adjacent literal markup is concatenated into `static_segment` and flushed as
/// a single write whenever a dynamic chunk (expression, control flow) appears.
pub(crate) struct ViewWriter {
    pub(self) chunks: Vec<Chunk>,
    static_segment: String,
    nested: bool,
    has_component: bool,
}

impl ViewWriter {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: false,
            has_component: false,
        }
    }

    pub fn new_nested() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: true,
            has_component: false,
        }
    }

    pub fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let mut static_segment = String::new();
            std::mem::swap(&mut self.static_segment, &mut static_segment);
            self.chunks.push(Chunk::Static {
                string: static_segment,
            });
        }
    }

    pub fn write_str_unescaped(&mut self, s: &str) {
        self.static_segment.push_str(s);
    }

    /// Appends literal text escaped for a text node position.
    pub fn write_text(&mut self, s: &str) {
        self.write_in_context(topcoat_view::HtmlContext::Text, s);
    }

    /// Appends literal text escaped for a double-quoted attribute value
    /// position.
    pub fn write_attribute_value(&mut self, s: &str) {
        self.write_in_context(topcoat_view::HtmlContext::AttributeValue, s);
    }

    fn write_in_context(&mut self, context: topcoat_view::HtmlContext, s: &str) {
        let mut f = topcoat_view::Formatter::new(&mut self.static_segment);
        context.writer(&mut f).write_str(s);
    }

    pub fn write_expr(&mut self, kind: ExprKind, tokens: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Expr { kind, tokens });
    }

    pub fn component(&mut self, tokens: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Component { tokens });
        self.has_component = true;
    }

    pub fn local_binding(&mut self, pat: &Pat, expr: &Expr) {
        // Local binding does not need flush because it does not have an effect on static segments.
        self.chunks.push(Chunk::Local {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
        });
    }

    pub fn statement(&mut self, tokens: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Statement { tokens });
    }

    pub fn for_loop(&mut self, pat: &Pat, expr: &Expr, f: impl FnOnce(&mut ViewWriter)) {
        self.flush();
        let mut body = ViewWriter::new();
        f(&mut body);
        body.flush();

        if body.has_component {
            self.has_component = true;
        }

        self.chunks.push(Chunk::For {
            pat: pat.clone(),
            expr: Box::new(expr.clone()),
            body: Box::new(body),
        });
    }

    pub fn if_else(&mut self, expr: &Expr, f: impl FnOnce(&mut ViewWriter, &mut ViewWriter)) {
        self.flush();
        let mut then_branch = ViewWriter::new();
        let mut else_branch = ViewWriter::new();
        f(&mut then_branch, &mut else_branch);
        then_branch.flush();
        else_branch.flush();

        if then_branch.has_component || else_branch.has_component {
            self.has_component = true;
        }

        self.chunks.push(Chunk::If {
            expr: expr.clone(),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        });
    }

    pub fn match_expr(&mut self, expr: &Expr, f: impl FnOnce(&mut MatchArmsBuilder)) {
        self.flush();
        let mut builder = MatchArmsBuilder { arms: Vec::new() };
        f(&mut builder);

        for arm in &builder.arms {
            if arm.body.has_component {
                self.has_component = true;
            }
        }

        self.chunks.push(Chunk::Match {
            expr: Box::new(expr.clone()),
            arms: builder.arms,
        });
    }

    pub fn into_token_stream(mut self) -> TokenStream {
        self.flush();

        let format_expr = {
            if self.chunks.is_empty() {
                // Optimized path: The view has no content.
                quote! { #topcoat_view::View::empty() }
            } else if self.chunks.len() == 1
                && let Chunk::Static { string } = &self.chunks[0]
            {
                quote! { #topcoat_view::View::unescaped_unchecked(#string) }
            } else {
                let mut emitter = ConcurrencyEmitter::new();
                emitter.build(&self)
            }
        };

        if self.nested {
            format_expr
        } else {
            quote! { async { ::core::result::Result::<#topcoat_view::View, #topcoat_error::Error>::Ok(#format_expr) }.await }
        }
    }
}

struct ConcurrencyEmitter {
    hoist: TokenStream,
    futures: Vec<Ident>,
    results: Vec<Ident>,
}

impl ConcurrencyEmitter {
    fn new() -> Self {
        Self {
            hoist: TokenStream::new(),
            futures: Vec::new(),
            results: Vec::new(),
        }
    }

    fn next_future(&mut self) -> (Ident, Ident) {
        let future = format_ident!("__fut{}", self.futures.len());
        let result = format_ident!("__res{}", self.results.len());
        self.futures.push(future.clone());
        self.results.push(result.clone());
        (future, result)
    }

    fn has_future(&self) -> bool {
        !self.futures.is_empty()
    }

    fn build_inner(&mut self, writer: &ViewWriter) -> TokenStream {
        let mut output = TokenStream::new();
        for chunk in &writer.chunks {
            match chunk {
                Chunk::Static { string } => {
                    let helper = ExprKind::Unescaped.helper();
                    let tokens = quote! { #string };
                    quote! { #helper(__cx, &mut __parts, #tokens); }.to_tokens(&mut output);
                }
                Chunk::Expr { kind, tokens } => {
                    let helper = kind.helper();
                    quote! { #helper(__cx, &mut __parts, #tokens); }.to_tokens(&mut output);
                }
                Chunk::Component { tokens } => {
                    let (future, result) = self.next_future();
                    quote! { let #future = async { #tokens }; }.to_tokens(&mut self.hoist);
                    quote! { __view(__cx, &mut __parts, #result); }.to_tokens(&mut output);
                }
                Chunk::Local { pat, expr } => {
                    quote! { let #pat = #expr; }.to_tokens(&mut self.hoist);
                }
                Chunk::Statement { .. } => {
                    todo!("statements currently are not supported");
                }
                Chunk::If {
                    expr,
                    then_branch: then,
                    else_branch: r#else,
                } => {
                    if then.has_component || r#else.has_component {
                        let then_branch = ConcurrencyEmitter::new().build(then);
                        let else_branch = ConcurrencyEmitter::new().build(r#else);
                        let else_branch =
                            (!r#else.chunks.is_empty()).then(|| quote! { else { #else_branch } });
                        let (future, result) = self.next_future();
                        quote! {
                            let #future: impl Future<Output = ::topcoat_core::error::Result<#topcoat_view::View>> = async {
                                if #expr {
                                    #then_branch
                                }
                                #else_branch
                            };
                        }
                        .to_tokens(&mut self.hoist);
                        quote! { __view(__cx, &mut __parts, #result); }.to_tokens(&mut output);
                    } else {
                        let then_branch = self.build_inner(then);
                        let else_branch = self.build_inner(r#else);
                        let else_branch =
                            (!r#else.chunks.is_empty()).then(|| quote! { else { #else_branch } });
                        quote! {
                            if #expr {
                                #then_branch
                            }
                            #else_branch
                        }
                        .to_tokens(&mut output);
                    }
                }
                Chunk::For { pat, expr, body } => {
                    if body.has_component {
                        let body = body.into_token_stream();
                        let (future, result) = self.next_future();
                        quote! {
                            let #future: impl Future<Output = ::topcoat_core::error::Result<#topcoat_view::View>> = {
                                #topcoat_view::internal::try_join_all((#expr).map(async |#pat| { #body }))
                            };
                        }
                        .to_tokens(&mut self.hoist);
                        quote! { __view(__cx, &mut __parts, #result); }.to_tokens(&mut output);
                    } else {
                        let body = self.build_inner(body);
                        quote! {
                            for #pat in #expr {
                                #body
                            }
                        }
                        .to_tokens(&mut output);
                    }
                }
                Chunk::Match { expr, arms } => {
                    // let arm_tokens = arms.iter().map(|arm| {
                    //     let pat = &arm.pat;
                    //     let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                    //     let body = build_parts(&arm.body.chunks);
                    //     quote! {
                    //         #pat #guard => { #body }
                    //     }
                    // });
                    // quote! {
                    //     match #expr {
                    //         #(#arm_tokens,)*
                    //     }
                    // }
                    // .to_tokens(&mut emit);
                }
            }
        }
        output
    }

    fn build(&mut self, writer: &ViewWriter) -> TokenStream {
        let inner = self.build_inner(writer);

        let hoist = &self.hoist;

        let futures = &self.futures;
        let results = &self.results;
        let join =
            quote! { let (#(#results),*) = #topcoat_view::internal::try_join!(#(#futures),*)?; };

        quote! {{
            use #topcoat_view::internal::*;
            let mut __parts = #topcoat_view::ViewParts::new();
            #hoist
            #join
            #inner
            #topcoat_view::View::new(__parts)
        }}
    }
}

/// Identifies which `internal` helper a [`Chunk::Expr`] should be wrapped in
/// when emitted, so the generated code uses the matching `__*` function and
/// the corresponding `*ViewParts` trait.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ExprKind {
    Unescaped,
    Node,
    ElementName,
    Attribute,
    AttributeUnescaped,
    AttributeKey,
    AttributeValue,
    Attributes,
}

impl ExprKind {
    fn helper(self) -> Ident {
        let name = match self {
            Self::Unescaped => "__unescaped",
            Self::Node => "__node",
            Self::ElementName => "__element_name",
            Self::Attribute => "__attribute",
            Self::AttributeUnescaped => "__attribute_unescaped",
            Self::AttributeKey => "__attribute_key",
            Self::AttributeValue => "__attribute_value",
            Self::Attributes => "__attributes",
        };
        Ident::new(name, Span::call_site())
    }
}

enum Chunk {
    Static {
        string: String,
    },
    Expr {
        kind: ExprKind,
        tokens: TokenStream,
    },
    Component {
        tokens: TokenStream,
    },
    Local {
        pat: Pat,
        expr: Box<Expr>,
    },
    Statement {
        tokens: TokenStream,
    },
    For {
        pat: Pat,
        expr: Box<Expr>,
        body: Box<ViewWriter>,
    },
    If {
        expr: Expr,
        then_branch: Box<ViewWriter>,
        else_branch: Box<ViewWriter>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

struct MatchArm {
    pat: Pat,
    guard: Option<Expr>,
    body: Box<ViewWriter>,
}

pub(crate) struct MatchArmsBuilder {
    arms: Vec<MatchArm>,
}

impl MatchArmsBuilder {
    pub fn arm(&mut self, pat: &Pat, guard: Option<&Expr>, f: impl FnOnce(&mut ViewWriter)) {
        let mut body = ViewWriter::new();
        f(&mut body);
        body.flush();
        self.arms.push(MatchArm {
            pat: pat.clone(),
            guard: guard.cloned(),
            body: Box::new(body),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(writer: ViewWriter) -> String {
        writer.into_token_stream().to_string()
    }

    #[test]
    fn empty_top_level_writer_emits_view_empty() {
        let out = rendered(ViewWriter::new());
        assert!(out.contains("async"));
        assert!(out.contains(&quote! { #topcoat_view::View::empty }.to_string()));
    }

    #[test]
    fn empty_nested_writer_omits_async_wrapper() {
        // Nested writers (e.g. component children) are spliced into a parent
        // and must not introduce their own async block.
        let out = rendered(ViewWriter::new_nested());
        assert!(!out.contains("async"));
        assert!(out.contains(&quote! { #topcoat_view::View::empty }.to_string()));
    }

    #[test]
    fn adjacent_literal_text_is_concatenated() {
        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<div>");
        writer.write_text("hello");
        writer.write_str_unescaped("</div>");
        let out = rendered(writer);
        assert!(out.contains("\"<div>hello</div>\""));
    }

    #[test]
    fn literal_text_is_escaped_for_its_position() {
        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<p>");
        writer.write_text("a < b & \"c\"");
        writer.write_str_unescaped("</p>");
        let out = rendered(writer);
        assert!(out.contains("a &lt; b &amp; \\\"c\\\""));

        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<p x=\"");
        writer.write_attribute_value("a < b & \"c\"");
        writer.write_str_unescaped("\">");
        let out = rendered(writer);
        assert!(out.contains("a < b &amp; &quot;c&quot;"));
    }

    #[test]
    fn expression_breaks_static_segment_with_kind_helper() {
        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<p>");
        writer.write_expr(ExprKind::Node, quote! { value });
        writer.write_str_unescaped("</p>");
        let out = rendered(writer);
        assert!(out.contains("__unescaped (__cx , & mut __parts , \"<p>\")"));
        assert!(out.contains("__node (__cx , & mut __parts , value)"));
        assert!(out.contains("__unescaped (__cx , & mut __parts , \"</p>\")"));
    }

    #[test]
    fn if_else_renders_both_branches() {
        let mut writer = ViewWriter::new();
        writer.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.write_str_unescaped("yes");
            else_branch.write_str_unescaped("no");
        });
        let out = rendered(writer);
        assert!(out.contains("if cond"));
        assert!(out.contains("else"));
        assert!(out.contains("\"yes\""));
        assert!(out.contains("\"no\""));
    }

    #[test]
    fn if_without_else_omits_else_branch() {
        let mut writer = ViewWriter::new();
        writer.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.write_str_unescaped("yes");
        });
        let out = rendered(writer);
        assert!(out.contains("if cond"));
        assert!(!out.contains("else"));
    }

    #[test]
    fn for_loop_wraps_body_in_for_in_expr() {
        let mut writer = ViewWriter::new();
        writer.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.write_str_unescaped("x");
        });
        let out = rendered(writer);
        assert!(out.contains("for x in xs"));
    }

    #[test]
    fn match_expr_renders_arms_with_optional_guard() {
        let mut writer = ViewWriter::new();
        writer.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                body.write_str_unescaped("a");
            });
            arms.arm(
                &syn::parse_quote!(B),
                Some(&syn::parse_quote!(flag)),
                |body| {
                    body.write_str_unescaped("b");
                },
            );
        });
        let out = rendered(writer);
        assert!(out.contains("match v"));
        assert!(out.contains("A =>"));
        assert!(out.contains("B if flag =>"));
    }

    #[test]
    fn local_binding_emits_let_statement() {
        let mut writer = ViewWriter::new();
        writer.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        writer.write_str_unescaped("ok");
        let out = rendered(writer);
        assert!(out.contains("let x = value"));
    }

    #[test]
    fn expr_kind_selects_matching_helper() {
        for (kind, expected) in [
            (ExprKind::Unescaped, "__unescaped"),
            (ExprKind::Node, "__node"),
            (ExprKind::Component, "__view"),
            (ExprKind::ElementName, "__element_name"),
            (ExprKind::Attribute, "__attribute"),
            (ExprKind::AttributeUnescaped, "__attribute_unescaped"),
            (ExprKind::AttributeKey, "__attribute_key"),
            (ExprKind::AttributeValue, "__attribute_value"),
            (ExprKind::Attributes, "__attributes"),
        ] {
            let mut writer = ViewWriter::new();
            writer.write_expr(kind, quote! { v });
            assert!(
                rendered(writer).contains(expected),
                "expected helper `{expected}` for kind `{expected}`",
            );
        }
    }
}
