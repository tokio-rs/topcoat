use syn::{
    LitStr,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use crate::{
    ast::{
        Component, DocumentType, Element, NodeBlock, NodeBreak, NodeContinue, NodeExpr,
        NodeForLoop, NodeIf, NodeLet, NodeMatch, ParseOption,
    },
    output::ViewWriter,
};

pub enum Node {
    Text(LitStr),
    DocumentType(DocumentType),
    Element(Element),
    Component(Component),
    Expr(NodeExpr),
    If(NodeIf),
    Let(NodeLet),
    ForLoop(NodeForLoop),
    Continue(NodeContinue),
    Break(NodeBreak),
    Match(NodeMatch),
    Block(NodeBlock),
}

impl Node {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Text(inner) => writer.write_str(&inner.value()),
            Self::DocumentType(inner) => inner.write(writer),
            Self::Element(inner) => inner.write(writer),
            Self::Component(inner) => inner.write(writer),
            Self::Expr(inner) => inner.write(writer),
            Self::If(inner) => inner.write(writer),
            Self::Let(inner) => inner.write(writer),
            Self::ForLoop(inner) => inner.write(writer),
            Self::Continue(inner) => inner.write(writer),
            Self::Break(inner) => inner.write(writer),
            Self::Match(inner) => inner.write(writer),
            Self::Block(inner) => inner.write(writer),
        }
    }
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = if input.peek(LitStr) {
            Self::Text(input.parse()?)
        } else if DocumentType::peek(input) {
            Self::DocumentType(input.parse()?)
        } else if Element::peek(input) {
            Self::Element(input.parse()?)
        } else if Component::peek(input) {
            Self::Component(input.parse()?)
        } else if NodeExpr::peek(input) {
            Self::Expr(input.parse()?)
        } else if NodeIf::peek(input) {
            Self::If(input.parse()?)
        } else if NodeLet::peek(input) {
            Self::Let(input.parse()?)
        } else if NodeForLoop::peek(input) {
            Self::ForLoop(input.parse()?)
        } else if NodeContinue::peek(input) {
            Self::Continue(input.parse()?)
        } else if NodeBreak::peek(input) {
            Self::Break(input.parse()?)
        } else if NodeMatch::peek(input) {
            Self::Match(input.parse()?)
        } else if NodeBlock::peek(input) {
            Self::Block(input.parse()?)
        } else {
            return Err(syn::Error::new(input.span(), "expected view node"));
        };

        match result {
            Self::Continue(inner) => Err(syn::Error::new(
                inner.expr_continue.span(),
                "`continue` is currently not supported",
            )),
            Self::Break(inner) => Err(syn::Error::new(
                inner.expr_break.span(),
                "`break` is currently not supported",
            )),
            _ => Ok(result),
        }
    }
}

#[cfg(feature = "pretty")]
impl crate::pretty::PrettyPrint for Node {
    fn pretty_print(&self, printer: &mut crate::pretty::Printer<'_>) {
        match self {
            Self::Text(inner) => inner.pretty_print(printer),
            _ => todo!("missing node formatting implementation"),
        }
    }
}
