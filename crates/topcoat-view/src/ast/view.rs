use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use crate::{ast::node::Node, output::ViewWriter};

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
        writer.to_tokens(tokens);
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
                printer.scan_break();
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
