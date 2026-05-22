use std::ops::Deref;

use syn::parse::{Parse, ParseStream};

use crate::ast::{
    ParseOption,
    view::{ClosingTag, Node, ViewWriter, WriteView},
};

/// A sequence of sibling [`Node`]s — the shared building block used by both a
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
impl topcoat_pretty::PrettyPrint for Nodes {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
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
