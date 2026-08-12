use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use topcoat_core_grammar::paths::topcoat_view;

use super::Node;

/// The lowered form of an [`Attributes`](crate::attributes::Attributes) list:
/// the HIR between the attribute AST and the emitted `TokenStream`. Built by
/// [`AttributeBuilder`](super::AttributeBuilder).
pub(crate) struct Scope {
    nodes: Vec<Node>,
}

impl Scope {
    pub(super) fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    /// The number of entries the emitted `Attributes` map is guaranteed to
    /// hold, used as its `with_capacity` hint.
    pub(super) fn capacity(&self) -> usize {
        self.nodes.iter().map(Node::capacity).sum()
    }

    /// Emits a block that builds the `Attributes` map.
    pub fn emit(&self) -> TokenStream {
        let capacity = self.capacity();
        let statements = Self::emit_nodes(&self.nodes);
        quote! {{
            let mut __attrs = #topcoat_view::Attributes::with_capacity(#capacity);
            #statements
            __attrs
        }}
    }

    fn emit_nodes(nodes: &[Node]) -> TokenStream {
        let mut output = TokenStream::new();
        for node in nodes {
            match node {
                Node::Insert { tokens, .. } | Node::Statement { tokens } => {
                    quote! { #tokens }
                }
                Node::Local { pat, expr } => {
                    quote! { let #pat = #expr; }
                }
                Node::For { pat, expr, body } => {
                    let body = Self::emit_nodes(&body.nodes);
                    quote! {
                        for #pat in #expr {
                            #body
                        }
                    }
                }
                Node::If {
                    cond,
                    then_branch,
                    else_branch,
                } => {
                    let then_tokens = Self::emit_nodes(&then_branch.nodes);
                    let else_tokens = (!else_branch.nodes.is_empty()).then(|| {
                        let tokens = Self::emit_nodes(&else_branch.nodes);
                        quote! { else { #tokens } }
                    });
                    quote! {
                        if #cond {
                            #then_tokens
                        }
                        #else_tokens
                    }
                }
                Node::Match { expr, arms } => {
                    let arm_tokens = arms.iter().map(|arm| {
                        let pat = &arm.pat;
                        let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                        let body = Self::emit_nodes(&arm.body.nodes);
                        quote! { #pat #guard => { #body } }
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
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;
    use crate::attributes::hir::AttributeBuilder;

    fn rendered(builder: AttributeBuilder) -> String {
        builder.finish().emit().to_string()
    }

    #[test]
    fn empty_view_emits_zero_capacity_block() {
        let builder = AttributeBuilder::new();
        let out = rendered(builder);
        assert!(
            out.contains(&quote! { #topcoat_view::Attributes::with_capacity(0usize) }.to_string())
        );
        assert!(!out.contains("insert"));
    }

    #[test]
    fn insert_records_one_capacity_per_entry() {
        let mut builder = AttributeBuilder::new();
        builder.insert(quote! { "class" }, quote! { "btn" });
        builder.insert(quote! { "id" }, quote! { "x" });
        let out = rendered(builder);
        assert!(out.contains("with_capacity (2usize)"));
        assert!(out.contains("__attrs . insert (__cx , \"class\" , \"btn\")"));
        assert!(out.contains("__attrs . insert (__cx , \"id\" , \"x\")"));
    }

    #[test]
    fn if_capacity_is_minimum_of_branches() {
        let mut builder = AttributeBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.insert(quote! { "a" }, quote! { "1" });
            then_branch.insert(quote! { "b" }, quote! { "2" });
            else_branch.insert(quote! { "c" }, quote! { "3" });
        });
        assert!(rendered(builder).contains("with_capacity (1usize)"));
    }

    #[test]
    fn if_without_else_contributes_no_minimum_capacity() {
        let mut builder = AttributeBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.insert(quote! { "a" }, quote! { "1" });
        });
        let out = rendered(builder);
        assert!(out.contains("with_capacity (0usize)"));
        assert!(!out.contains("else"));
    }

    #[test]
    fn for_loop_contributes_no_static_capacity() {
        let mut builder = AttributeBuilder::new();
        builder.for_loop(
            &syn::parse_quote!((k, v)),
            &syn::parse_quote!(items),
            |body| body.insert(quote! { k }, quote! { v }),
        );
        let out = rendered(builder);
        assert!(out.contains("with_capacity (0usize)"));
        assert!(out.contains("for (k , v) in items"));
    }

    #[test]
    fn match_capacity_is_minimum_over_arms() {
        let mut builder = AttributeBuilder::new();
        builder.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                body.insert(quote! { "x" }, quote! { "1" });
            });
            arms.arm(
                &syn::parse_quote!(B),
                Some(&syn::parse_quote!(flag)),
                |body| {
                    body.insert(quote! { "x" }, quote! { "2" });
                    body.insert(quote! { "y" }, quote! { "3" });
                },
            );
        });
        let out = rendered(builder);
        assert!(out.contains("with_capacity (1usize)"));
        assert!(out.contains("match v"));
        assert!(out.contains("if flag"));
    }

    #[test]
    fn let_and_statement_are_emitted_verbatim() {
        let mut builder = AttributeBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        builder.statement(quote! { break; });
        let out = rendered(builder);
        assert!(out.contains("let x = value"));
        assert!(out.contains("break ;"));
    }
}
