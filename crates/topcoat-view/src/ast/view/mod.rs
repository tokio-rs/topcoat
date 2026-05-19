mod attribute;
mod attribute_key;
mod attribute_node;
mod attribute_value;
mod attributes;
mod component;
mod component_tag;
mod document_type;
mod element;
mod element_name;
mod element_tag;
mod html_ident;
mod node;
mod reactive_scope;
mod signal_declaration;
mod template_block;
mod template_expr;
mod template_for_loop;
mod template_if;
mod template_let;
mod template_match;
mod view_writer;

pub use attribute::*;
pub use attribute_key::*;
pub use attribute_node::*;
pub use attribute_value::*;
pub use attributes::*;
pub use component::*;
pub use component_tag::*;
pub use document_type::*;
pub use element::*;
pub use element_name::*;
pub use element_tag::*;
pub use html_ident::*;
pub use node::*;
pub use reactive_scope::*;
pub use signal_declaration::*;
pub use template_block::*;
pub use template_expr::*;
pub use template_for_loop::*;
pub use template_if::*;
pub use template_let::*;
pub use template_match::*;
pub(crate) use view_writer::*;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use crate::ast::view::{Node, ViewWriter, WriteView};

/// The parsed body of a `view!` invocation. Lowers to a
/// [`runtime::View`](crate::runtime::View).
pub struct View {
    pub nodes: Vec<Node>,
}

impl Parse for View {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            nodes: {
                let mut children = Vec::new();
                while !input.is_empty() {
                    children.push(input.parse()?)
                }
                children
            },
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
        // Nodes in a view are simply space separated, or line separated if there is not enough
        // space horizontally.
        for (index, node) in self.nodes.iter().enumerate() {
            node.pretty_print(printer);
            if index < self.nodes.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
