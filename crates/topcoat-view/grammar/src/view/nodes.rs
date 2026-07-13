use std::ops::Deref;

use syn::parse::{Parse, ParseStream};

use topcoat_core_grammar::ParseOption;

use crate::view::{ClosingTag, Node, ViewWriter, WriteView};

/// A sequence of sibling [`Node`]s: the shared building block used by both a
/// top-level [`View`](super::View), a [`Component`](super::Component)'s
/// children, and any view-position template body. Owns the node list and the
/// formatting rules for laying out siblings.
pub struct Nodes(Vec<Node>);

impl Deref for Nodes {
    type Target = [Node];

    fn deref(&self) -> &[Node] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a Nodes {
    type Item = &'a Node;
    type IntoIter = std::slice::Iter<'a, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Parse for Nodes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() && !ClosingTag::peek(input) {
            nodes.push(input.parse()?);
        }
        Ok(Self(nodes))
    }
}

impl WriteView for Nodes {
    fn write(&self, writer: &mut ViewWriter) {
        for node in self {
            node.write(writer);
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Nodes {
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
    use syn::parse::Parser;

    fn parse(source: &str) -> Nodes {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_empty_input() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn collects_sibling_nodes_in_order() {
        let nodes = parse(r#""a" (b) <span>"c"</span>"#);
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0], Node::Text(_)));
        assert!(matches!(nodes[1], Node::Expr(_)));
        assert!(matches!(nodes[2], Node::Element(_)));
    }

    #[test]
    fn stops_at_closing_tag_without_consuming_it() {
        let parser = |input: syn::parse::ParseStream| -> syn::Result<(Nodes, ClosingTag)> {
            let nodes = input.parse::<Nodes>()?;
            let closing = input.parse::<ClosingTag>()?;
            Ok((nodes, closing))
        };
        let (nodes, closing) = parser.parse_str(r#""a" "b" </div>"#).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(closing.name.string_name().as_deref(), Some("div"));
    }
}
