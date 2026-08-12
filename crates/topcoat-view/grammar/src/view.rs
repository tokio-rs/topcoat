mod component;
mod document_type;
mod element;
mod element_name;
mod element_tag;
pub(crate) mod hir;
mod html_ident;
mod node;
mod nodes;
mod signal_declaration;

pub use component::*;
pub use document_type::*;
pub use element::*;
pub use element_name::*;
pub use element_tag::*;
pub use html_ident::*;
pub use node::*;
pub use nodes::*;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
pub use signal_declaration::*;
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::ParseOption;

use crate::{
    leading_cx::LeadingCx,
    view::hir::{LowerView, ViewBuilder},
};

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::Markup`](topcoat_view::Markup).
pub struct View {
    /// The request context binding supplied by a leading `cx =>` argument.
    ///
    /// Inside a `#[component]`, `#[page]`, `#[layout]`, or `#[shard]`, the
    /// context is available implicitly, so this is [`None`]. Anywhere else
    /// (for example a `#[route]` handler), the caller names it explicitly as
    /// `view! { cx => ... }` and the rest of the view renders against it.
    pub cx: Option<LeadingCx>,
    pub nodes: Nodes,
}

impl Parse for View {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cx: input.call(LeadingCx::parse_option)?,
            nodes: input.parse()?,
        })
    }
}

impl ToTokens for View {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // A standalone `view!` has no render loop to live in: the component
        // macros splice the pass expansion over the returning `view!` before
        // the body re-emits, so an invocation reaching this expansion sits
        // in a position the component future model does not support.
        let _ = &self.nodes;
        quote! {
            ::core::compile_error!(
                "`view!` is only supported as the returning expression of a \
                 `#[component]`, `#[page]`, or `#[layout]` function"
            )
        }
        .to_tokens(tokens);
    }
}

impl View {
    /// Emits this view as a [`Markup`](topcoat_view::Markup) value: the
    /// component free form behind `markup!`, for fragments that are data
    /// rather than render code (mail bodies, patch payloads).
    #[must_use]
    pub fn emit_markup(&self) -> TokenStream {
        let mut builder = ViewBuilder::new();
        self.nodes.lower(&mut builder);
        let scope = builder.finish();
        if scope.is_async() {
            return quote! {
                ::core::compile_error!(
                    "components render through passes and cannot appear in \
                     `markup!`; build plain markup here"
                )
            };
        }
        let markup = scope.emit_root();
        match &self.cx {
            Some(cx) => quote! {{ #cx #markup }},
            None => markup,
        }
    }

    /// Emits this view as a component's render loop: the pass expansion the
    /// component macros splice over a returning `view!`.
    #[must_use]
    pub fn emit_pass(&self) -> TokenStream {
        self.emit_pass_with_tokens(std::collections::HashSet::new())
    }

    /// Like [`View::emit_pass`], lowering bare interpolations of the named
    /// identifiers as view token placements.
    #[must_use]
    pub fn emit_pass_with_tokens(&self, tokens: std::collections::HashSet<String>) -> TokenStream {
        let mut builder = ViewBuilder::with_tokens(tokens);
        self.nodes.lower(&mut builder);
        let root = builder.finish().emit_pass_root();

        // When an explicit context is named, bind it to the `__cx` identifier
        // the generated code reads. Inside a component this binding is
        // already in scope.
        match &self.cx {
            Some(cx) => quote! {{ #cx #root }},
            None => root,
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for View {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        if let Some(cx) = &self.cx {
            cx.pretty_print(printer);
        }
        self.nodes.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Node;

    fn parse(source: &str) -> View {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn empty_input_yields_no_nodes() {
        assert!(parse("").nodes.is_empty());
    }

    #[test]
    fn collects_sibling_nodes_in_order() {
        let view = parse(r#""a" "b" "c""#);
        assert_eq!(view.nodes.len(), 3);
        assert!(view.nodes.iter().all(|n| matches!(n, Node::Text(_))));
    }

    #[test]
    fn parses_leading_cx_argument() {
        let view = parse("cx => <div></div>");
        assert_eq!(view.cx.map(|cx| cx.cx.to_string()), Some("cx".to_owned()));
        assert_eq!(view.nodes.len(), 1);
    }

    #[test]
    fn omitted_cx_is_none() {
        assert!(parse("<div></div>").cx.is_none());
    }

    #[test]
    fn component_invocation_is_not_mistaken_for_cx() {
        // A component invocation also starts with an identifier, but it is
        // followed by `(`, not `=>`, so it stays a node.
        let view = parse(r#"greeting(name: "World")"#);
        assert!(view.cx.is_none());
        assert_eq!(view.nodes.len(), 1);
    }

    #[test]
    fn leading_text_nodes_are_not_mistaken_for_cx() {
        // A leading string literal is not an identifier, so it is never consumed
        // as a `cx` argument.
        let view = parse(r#""a" "b""#);
        assert!(view.cx.is_none());
        assert_eq!(view.nodes.len(), 2);
    }

    #[test]
    fn explicit_cx_binds_the_context_identifier() {
        let tokens = parse("cx => <div></div>").emit_markup().to_string();
        assert!(tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn omitted_cx_emits_no_binding() {
        let tokens = parse("<div></div>").emit_markup().to_string();
        assert!(!tokens.contains("let __cx"), "{tokens}");
    }
}
