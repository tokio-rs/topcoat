use syn::{
    LitStr,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use topcoat_core_grammar::ParseOption;

use crate::{
    template::{
        MatchArmBody, RuntimeExpr, TemplateBlock, TemplateBreak, TemplateContinue, TemplateExpr,
        TemplateForLoop, TemplateIf, TemplateLet, TemplateMatch,
    },
    view::{
        Component, DocumentType, Element, Nodes, ReactiveScope, SignalDeclaration, ViewWriter,
        WriteView,
    },
};

/// A single child within a [`View`](super::View): the union of every construct
/// that can appear at view-body position.
pub enum Node {
    Text(LitStr),
    DocumentType(DocumentType),
    Element(Box<Element>),
    Component(Component),
    Expr(TemplateExpr),
    RuntimeExpr(RuntimeExpr),
    If(TemplateIf<Nodes>),
    Let(TemplateLet),
    ForLoop(TemplateForLoop<Nodes>),
    Continue(TemplateContinue),
    Break(TemplateBreak),
    Match(TemplateMatch<Node>),
    Block(TemplateBlock<Nodes>),
    SignalDecaration(SignalDeclaration),
    ReactiveScope(ReactiveScope),
}

impl Node {
    /// Returns `true` if the node is [`Block`].
    ///
    /// [`Block`]: Node::Block
    #[must_use]
    pub fn is_block(&self) -> bool {
        matches!(self, Self::Block(..))
    }
}

impl MatchArmBody for Node {
    fn is_block_body(&self) -> bool {
        self.is_block()
    }
}

impl WriteView for Node {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Text(inner) => writer.write_text(&inner.value()),
            Self::DocumentType(inner) => inner.write(writer),
            Self::Element(inner) => inner.write(writer),
            Self::Component(inner) => inner.write(writer),
            Self::Expr(inner) => inner.write(writer),
            Self::RuntimeExpr(inner) => inner.write(writer),
            Self::If(inner) => inner.write(writer),
            Self::Let(inner) => inner.write(writer),
            Self::ForLoop(inner) => inner.write(writer),
            Self::Continue(inner) => inner.write(writer),
            Self::Break(inner) => inner.write(writer),
            Self::Match(inner) => inner.write(writer),
            Self::Block(inner) => inner.write(writer),
            Self::SignalDecaration(inner) => inner.write(writer),
            Self::ReactiveScope(inner) => inner.write(writer),
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
        } else if TemplateExpr::peek(input) {
            Self::Expr(input.parse()?)
        } else if RuntimeExpr::peek(input) {
            Self::RuntimeExpr(input.parse()?)
        } else if TemplateIf::<Nodes>::peek(input) {
            Self::If(input.parse()?)
        } else if TemplateLet::peek(input) {
            Self::Let(input.parse()?)
        } else if TemplateForLoop::<Nodes>::peek(input) {
            Self::ForLoop(input.parse()?)
        } else if TemplateContinue::peek(input) {
            Self::Continue(input.parse()?)
        } else if TemplateBreak::peek(input) {
            Self::Break(input.parse()?)
        } else if TemplateMatch::<Node>::peek(input) {
            Self::Match(input.parse()?)
        } else if TemplateBlock::<Nodes>::peek(input) {
            Self::Block(input.parse()?)
        } else if SignalDeclaration::peek(input) {
            Self::SignalDecaration(input.parse()?)
        } else if ReactiveScope::peek(input) {
            Self::ReactiveScope(input.parse()?)
        } else if Component::peek(input) {
            Self::Component(input.parse()?)
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
impl topcoat_core_grammar::pretty::PrettyPrint for Node {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Text(inner) => inner.pretty_print(printer),
            Self::DocumentType(inner) => inner.pretty_print(printer),
            Self::Element(inner) => inner.pretty_print(printer),
            Self::Component(inner) => inner.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
            Self::RuntimeExpr(inner) => inner.pretty_print(printer),
            Self::If(inner) => inner.pretty_print(printer),
            Self::Let(inner) => inner.pretty_print(printer),
            Self::ForLoop(inner) => inner.pretty_print(printer),
            Self::Continue(inner) => inner.pretty_print(printer),
            Self::Break(inner) => inner.pretty_print(printer),
            Self::Match(inner) => inner.pretty_print(printer),
            Self::Block(inner) => inner.pretty_print(printer),
            Self::SignalDecaration(inner) => inner.pretty_print(printer),
            Self::ReactiveScope(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Node {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Node>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn dispatches_each_variant() {
        assert!(matches!(parse(r#""hi""#), Node::Text(_)));
        assert!(matches!(parse("<!DOCTYPE html>"), Node::DocumentType(_)));
        assert!(matches!(parse("<br>"), Node::Element(_)));
        assert!(matches!(parse("foo()"), Node::Component(_)));
        assert!(matches!(parse("(value)"), Node::Expr(_)));
        assert!(matches!(parse(r#"if a { "x" }"#), Node::If(_)));
        assert!(matches!(parse(r"let a = 1;"), Node::Let(_)));
        assert!(matches!(parse(r"for x in xs { (x) }"), Node::ForLoop(_)));
        assert!(matches!(parse(r#"match v { _ => "x", }"#), Node::Match(_)));
        assert!(matches!(parse(r#"{ "x" }"#), Node::Block(_)));
    }

    #[test]
    fn break_in_loop_is_rejected() {
        assert!(parse_err("break;").contains("`break` is currently not supported"));
    }

    #[test]
    fn continue_in_loop_is_rejected() {
        assert!(parse_err("continue;").contains("`continue` is currently not supported"));
    }

    #[test]
    fn unrecognized_token_is_rejected() {
        assert!(parse_err("@").contains("expected view node"));
    }

    #[test]
    fn is_block_only_true_for_block_variant() {
        assert!(parse(r#"{ "x" }"#).is_block());
        assert!(!parse(r#""x""#).is_block());
    }
}
