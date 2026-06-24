use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Expr, Ident, Pat};

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
}

impl ViewWriter {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: false,
        }
    }

    pub fn new_nested() -> Self {
        Self {
            chunks: Vec::new(),
            static_segment: String::new(),
            nested: true,
        }
    }

    pub fn flush(&mut self) {
        if !self.static_segment.is_empty() {
            let static_segment = &self.static_segment;
            self.chunks.push(Chunk::Expr {
                kind: ExprKind::Unescaped,
                tokens: quote! { #static_segment },
            });
            self.static_segment.clear();
        }
    }

    pub fn write_str_unescaped(&mut self, s: &str) {
        self.static_segment.push_str(s);
    }

    pub fn write_str(&mut self, s: &str) {
        crate::runtime::Formatter::new(&mut self.static_segment).write_str(s);
    }

    pub fn write_expr(&mut self, kind: ExprKind, tokens: TokenStream) {
        self.flush();
        self.chunks.push(Chunk::Expr { kind, tokens });
    }

    pub fn let_binding(&mut self, pat: &Pat, expr: &Expr) {
        self.flush();
        self.chunks.push(Chunk::Let {
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
                quote! { ::topcoat::view::View::empty() }
            } else {
                fn build_parts(chunks: &[Chunk]) -> TokenStream {
                    let mut output = TokenStream::new();
                    for chunk in chunks {
                        match chunk {
                            Chunk::Expr { kind, tokens } => {
                                let helper = kind.helper();
                                quote! { #helper(&mut __parts, #tokens); }
                            }
                            Chunk::Let { pat, expr } => {
                                quote! { let #pat = #expr; }
                            }
                            Chunk::Statement { tokens } => {
                                quote! { #tokens }
                            }
                            Chunk::If {
                                expr,
                                then_branch: then,
                                else_branch: r#else,
                            } => {
                                let then_branch = build_parts(&then.chunks);
                                let else_branch = build_parts(&r#else.chunks);
                                let else_branch = (!r#else.chunks.is_empty())
                                    .then(|| quote! { else { #else_branch } });
                                quote! {
                                    if #expr {
                                        #then_branch
                                    }
                                    #else_branch
                                }
                            }
                            Chunk::For { pat, expr, body } => {
                                let body = build_parts(&body.chunks);
                                quote! {
                                    for #pat in #expr {
                                        #body
                                    }
                                }
                            }
                            Chunk::Match { expr, arms } => {
                                let arm_tokens = arms.iter().map(|arm| {
                                    let pat = &arm.pat;
                                    let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                                    let body = build_parts(&arm.body.chunks);
                                    quote! {
                                        #pat #guard => { #body }
                                    }
                                });
                                quote! {
                                    match #expr {
                                        #(#arm_tokens,)*
                                    }
                                }
                            }
                        }
                        .to_tokens(&mut output);
                    }
                    output
                }

                let statements = build_parts(&self.chunks);

                quote! {{
                    use ::topcoat::view::internal::*;
                    let mut __parts = ::topcoat::view::ViewParts::new();
                    #statements
                    ::topcoat::view::View::new(__parts)
                }}
            }
        };

        if self.nested {
            format_expr
        } else {
            quote! { async { ::core::result::Result::<::topcoat::view::View, ::topcoat::Error>::Ok(#format_expr) }.await }
        }
    }
}

/// Identifies which `internal` helper a [`Chunk::Expr`] should be wrapped in
/// when emitted, so the generated code uses the matching `__*` function and
/// the corresponding `*ViewParts` trait.
#[derive(Copy, Clone)]
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
    Expr {
        kind: ExprKind,
        tokens: TokenStream,
    },
    Let {
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
        assert!(out.contains(":: topcoat :: view :: View :: empty"));
    }

    #[test]
    fn empty_nested_writer_omits_async_wrapper() {
        // Nested writers (e.g. component children) are spliced into a parent
        // and must not introduce their own async block.
        let out = rendered(ViewWriter::new_nested());
        assert!(!out.contains("async"));
        assert!(out.contains(":: topcoat :: view :: View :: empty"));
    }

    #[test]
    fn adjacent_literal_text_is_concatenated() {
        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<div>");
        writer.write_str("hello");
        writer.write_str_unescaped("</div>");
        let out = rendered(writer);
        assert!(out.contains("__unescaped (& mut __parts , \"<div>hello</div>\")"));
    }

    #[test]
    fn expression_breaks_static_segment_with_kind_helper() {
        let mut writer = ViewWriter::new();
        writer.write_str_unescaped("<p>");
        writer.write_expr(ExprKind::Node, quote! { value });
        writer.write_str_unescaped("</p>");
        let out = rendered(writer);
        assert!(out.contains("__unescaped (& mut __parts , \"<p>\")"));
        assert!(out.contains("__node (& mut __parts , value)"));
        assert!(out.contains("__unescaped (& mut __parts , \"</p>\")"));
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
    fn let_binding_emits_let_statement() {
        let mut writer = ViewWriter::new();
        writer.let_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        writer.write_str_unescaped("ok");
        let out = rendered(writer);
        assert!(out.contains("let x = value"));
    }

    #[test]
    fn expr_kind_selects_matching_helper() {
        for (kind, expected) in [
            (ExprKind::Unescaped, "__unescaped"),
            (ExprKind::Node, "__node"),
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
