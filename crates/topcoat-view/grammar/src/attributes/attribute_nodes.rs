use std::ops::Deref;

use syn::parse::{Parse, ParseStream};

use crate::{
    attributes::{AttributeNode, AttributeWriter, WriteAttribute},
    view::{ViewWriter, WriteView},
};

/// A sequence of sibling [`AttributeNode`]s: the attribute-position counterpart
/// of [`Nodes`](crate::view::Nodes). Used as the body of attribute-position template
/// constructs (`if`/`for`/`match` inside an opening tag's attribute list).
pub struct AttributeNodes(Vec<AttributeNode>);

impl Deref for AttributeNodes {
    type Target = [AttributeNode];

    fn deref(&self) -> &[AttributeNode] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a AttributeNodes {
    type Item = &'a AttributeNode;
    type IntoIter = std::slice::Iter<'a, AttributeNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Parse for AttributeNodes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(input.parse()?);
        }
        Ok(Self(nodes))
    }
}

impl WriteView for AttributeNodes {
    fn write(&self, writer: &mut ViewWriter) {
        for node in self {
            WriteView::write(node, writer);
        }
    }
}

impl WriteAttribute for AttributeNodes {
    fn write(&self, writer: &mut AttributeWriter) {
        for node in self {
            WriteAttribute::write(node, writer);
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for AttributeNodes {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        for (index, node) in self.iter().enumerate() {
            node.pretty_print(printer);
            if index < self.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
        if self.len() > 1 {
            printer.scan_force_break();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> AttributeNodes {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_empty_input() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn collects_sibling_nodes_without_separators() {
        let nodes = parse(r#"class="x" id="y" data-z="z""#);
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn collects_mixed_node_kinds() {
        let nodes = parse(r#"class="x" :value=(v) @click="alert()""#);
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], AttributeNode::Attribute(_)));
        assert!(matches!(nodes[1], AttributeNode::BindAttribute(_)));
        assert!(matches!(nodes[2], AttributeNode::EventHandler(_)));
    }
}
