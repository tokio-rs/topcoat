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
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::View`](crate::runtime::View).
pub struct View {
    pub nodes: Nodes,
}

impl Parse for View {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
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
        writer.into_token_stream().to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for View {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.nodes.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::view::Node;

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
}
