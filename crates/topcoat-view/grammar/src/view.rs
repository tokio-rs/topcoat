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
use topcoat_core_grammar::{ParseOption, paths::topcoat_context};

use crate::{
    leading_cx::LeadingCx,
    view::hir::{LowerView, ViewBuilder},
};

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::View`](topcoat_view::View).
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

impl View {
    /// Expands the view to the expression evaluating to its value.
    ///
    /// A self-contained view builds in a buffer of its own and seals its
    /// content into it, so what it resolves to renders anywhere. Otherwise
    /// the view builds into the ambient `__buf` buffer of its scope and
    /// resolves to a handle into it. A view naming its context is always
    /// self-contained, since it may be built outside any scope providing a
    /// buffer.
    #[must_use]
    pub fn expand(&self, self_contained: bool) -> TokenStream {
        let mut builder = ViewBuilder::new();
        self.nodes.lower(&mut builder);
        let owns_cx = self.cx.is_some();
        let view = builder
            .finish()
            .emit_root(owns_cx, self_contained || owns_cx);

        // When an explicit context is named, the view owns a copy of it
        // rather than borrowing it, so the view can outlive the binding it
        // was named from. Inside a component/page/layout the `__cx` binding
        // is already in scope, so the view is emitted untouched.
        match &self.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #view
                }}
            }
            None => view,
        }
    }
}

impl ToTokens for View {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expand(false).to_tokens(tokens);
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
        let tokens = parse("cx => <div></div>").to_token_stream().to_string();
        assert!(tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn omitted_cx_emits_no_binding() {
        let tokens = parse("<div></div>").to_token_stream().to_string();
        assert!(!tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn explicit_cx_builds_in_its_own_buffer() {
        let tokens = parse("cx => <div></div>").to_token_stream().to_string();
        assert!(tokens.contains("let __buf"), "{tokens}");
        assert!(tokens.contains("drive_sealed (__buf , __view)"), "{tokens}");
    }

    #[test]
    fn omitted_cx_builds_in_the_ambient_buffer() {
        let tokens = parse("<div></div>").to_token_stream().to_string();
        assert!(!tokens.contains("let __buf"), "{tokens}");
        assert!(tokens.contains("drive (__view)"), "{tokens}");
    }

    #[test]
    fn a_self_contained_expansion_keeps_the_ambient_cx() {
        let tokens = parse("<div></div>").expand(true).to_string();
        assert!(!tokens.contains("let __cx"), "{tokens}");
        assert!(tokens.contains("let __buf"), "{tokens}");
        assert!(tokens.contains("drive_sealed (__buf , __view)"), "{tokens}");
    }
}
