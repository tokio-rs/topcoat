mod component;
mod document_type;
mod element;
mod element_name;
mod element_tag;
mod html_ident;
mod node;
mod nodes;
mod reactive_scope;
mod signal_declaration;
mod view_writer;

pub use component::*;
pub use document_type::*;
pub use element::*;
pub use element_name::*;
pub use element_tag::*;
pub use html_ident::*;
pub use node::*;
pub use nodes::*;
pub use reactive_scope::*;
pub use signal_declaration::*;
pub(crate) use view_writer::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};

use topcoat_core_grammar::ParseOption;

use crate::leading_cx::LeadingCx;

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::View`](topcoat_view::View).
pub struct View {
    /// The request context binding supplied by a leading `cx =>` argument.
    ///
    /// Inside a `#[component]`, `#[page]`, or `#[layout]`, the context is
    /// available implicitly, so this is [`None`]. Anywhere else (for example a
    /// `#[route]` handler), the caller names it explicitly as `view! { cx => ... }`
    /// and the rest of the view renders against it.
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
        let mut writer = ViewWriter::new();
        for node in &self.nodes {
            node.write(&mut writer);
        }
        let view = writer.into_token_stream();

        // When an explicit context is named, bind it to the `__cx` identifier
        // the generated code (component invocations, reactive scopes) reads
        // from. Inside a component/page/layout this binding is already in scope,
        // so we emit the view untouched.
        match &self.cx {
            Some(cx) => quote! {
                {
                    #cx
                    #view
                }
            }
            .to_tokens(tokens),
            None => view.to_tokens(tokens),
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
        let tokens = parse("cx => <div></div>").to_token_stream().to_string();
        assert!(tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn omitted_cx_emits_no_binding() {
        let tokens = parse("<div></div>").to_token_stream().to_string();
        assert!(!tokens.contains("let __cx"), "{tokens}");
    }
}
