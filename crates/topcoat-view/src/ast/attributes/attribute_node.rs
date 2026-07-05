use syn::parse::{Parse, ParseStream};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{
        Attribute, AttributeNodes, AttributeSpread, AttributeWriter, BindAttribute, EventHandler,
        WriteAttribute,
    },
    template::{
        MatchArmBody, TemplateBlock, TemplateBreak, TemplateContinue, TemplateForLoop, TemplateIf,
        TemplateLet, TemplateMatch,
    },
    view::{ViewWriter, WriteView},
};

/// A single entry within an [`Attributes`](super::Attributes) list: the union
/// of every construct that can appear at attribute-list position.
pub enum AttributeNode {
    Attribute(Attribute),
    Spread(AttributeSpread),
    BindAttribute(Box<BindAttribute>),
    EventHandler(EventHandler),
    If(Box<TemplateIf<AttributeNodes>>),
    Let(TemplateLet),
    ForLoop(TemplateForLoop<AttributeNodes>),
    Continue(TemplateContinue),
    Break(TemplateBreak),
    Match(TemplateMatch<AttributeNode>),
    Block(TemplateBlock<AttributeNodes>),
}

impl MatchArmBody for AttributeNode {
    fn is_block_body(&self) -> bool {
        matches!(self, Self::Block(..))
    }
}

impl WriteView for AttributeNode {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Attribute(inner) => WriteView::write(inner, writer),
            Self::Spread(inner) => WriteView::write(inner, writer),
            Self::BindAttribute(inner) => WriteView::write(inner.as_ref(), writer),
            Self::EventHandler(inner) => WriteView::write(inner, writer),
            Self::If(inner) => WriteView::write(inner.as_ref(), writer),
            Self::Let(inner) => WriteView::write(inner, writer),
            Self::ForLoop(inner) => WriteView::write(inner, writer),
            Self::Continue(inner) => WriteView::write(inner, writer),
            Self::Break(inner) => WriteView::write(inner, writer),
            Self::Match(inner) => WriteView::write(inner, writer),
            Self::Block(inner) => WriteView::write(inner, writer),
        }
    }
}

impl WriteAttribute for AttributeNode {
    fn write(&self, writer: &mut AttributeWriter) {
        match self {
            Self::Attribute(inner) => WriteAttribute::write(inner, writer),
            Self::Spread(inner) => WriteAttribute::write(inner, writer),
            Self::BindAttribute(inner) => WriteAttribute::write(inner.as_ref(), writer),
            Self::EventHandler(inner) => WriteAttribute::write(inner, writer),
            Self::If(inner) => WriteAttribute::write(inner.as_ref(), writer),
            Self::Let(inner) => WriteAttribute::write(inner, writer),
            Self::ForLoop(inner) => WriteAttribute::write(inner, writer),
            Self::Continue(inner) => WriteAttribute::write(inner, writer),
            Self::Break(inner) => WriteAttribute::write(inner, writer),
            Self::Match(inner) => WriteAttribute::write(inner, writer),
            Self::Block(inner) => WriteAttribute::write(inner, writer),
        }
    }
}

impl Parse for AttributeNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = if TemplateIf::<AttributeNodes>::peek(input) {
            Self::If(input.parse()?)
        } else if TemplateLet::peek(input) {
            Self::Let(input.parse()?)
        } else if TemplateForLoop::<AttributeNodes>::peek(input) {
            Self::ForLoop(input.parse()?)
        } else if TemplateContinue::peek(input) {
            Self::Continue(input.parse()?)
        } else if TemplateBreak::peek(input) {
            Self::Break(input.parse()?)
        } else if TemplateMatch::<AttributeNode>::peek(input) {
            Self::Match(input.parse()?)
        } else if TemplateBlock::<AttributeNodes>::peek(input) {
            Self::Block(input.parse()?)
        } else if BindAttribute::peek(input) {
            Self::BindAttribute(input.parse()?)
        } else if EventHandler::peek(input) {
            Self::EventHandler(input.parse()?)
        } else if AttributeSpread::peek(input) {
            Self::Spread(input.parse()?)
        } else if Attribute::peek(input) {
            Self::Attribute(input.parse()?)
        } else {
            return Err(syn::Error::new(input.span(), "expected attribute node"));
        };

        Ok(result)
    }
}

impl ParseOption for AttributeNode {
    fn peek(input: ParseStream) -> bool {
        Attribute::peek(input)
            || AttributeSpread::peek(input)
            || BindAttribute::peek(input)
            || EventHandler::peek(input)
            || TemplateIf::<AttributeNodes>::peek(input)
            || TemplateLet::peek(input)
            || TemplateForLoop::<AttributeNodes>::peek(input)
            || TemplateContinue::peek(input)
            || TemplateBreak::peek(input)
            || TemplateMatch::<AttributeNode>::peek(input)
            || TemplateBlock::<AttributeNodes>::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for AttributeNode {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Attribute(inner) => inner.pretty_print(printer),
            Self::Spread(inner) => inner.pretty_print(printer),
            Self::BindAttribute(inner) => inner.pretty_print(printer),
            Self::EventHandler(inner) => inner.pretty_print(printer),
            Self::If(inner) => inner.pretty_print(printer),
            Self::Let(inner) => inner.pretty_print(printer),
            Self::ForLoop(inner) => inner.pretty_print(printer),
            Self::Continue(inner) => inner.pretty_print(printer),
            Self::Break(inner) => inner.pretty_print(printer),
            Self::Match(inner) => inner.pretty_print(printer),
            Self::Block(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> AttributeNode {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn dispatches_each_variant() {
        assert!(matches!(parse(r#"foo="bar""#), AttributeNode::Attribute(_)));
        assert!(matches!(
            parse(r":foo=(bar)"),
            AttributeNode::BindAttribute(_),
        ));
        assert!(matches!(
            parse(r":foo=$(bar)"),
            AttributeNode::BindAttribute(_),
        ));
        assert!(matches!(
            parse(r"@foo=(bar)"),
            AttributeNode::EventHandler(_),
        ));
        assert!(matches!(
            parse(r"@foo=$(bar)"),
            AttributeNode::EventHandler(_),
        ));
        assert!(matches!(
            parse(r#"@foo="bar()""#),
            AttributeNode::EventHandler(_),
        ));
        assert!(matches!(
            parse(r#"if cond { foo="bar" }"#),
            AttributeNode::If(_),
        ));
        assert!(matches!(parse("let a = 1;"), AttributeNode::Let(_)));
        assert!(matches!(
            parse(r#"for x in xs { foo="bar" }"#),
            AttributeNode::ForLoop(_),
        ));
        assert!(matches!(parse("break;"), AttributeNode::Break(_)));
        assert!(matches!(parse("continue;"), AttributeNode::Continue(_)));
        assert!(matches!(
            parse(r#"match v { _ => foo="bar", }"#),
            AttributeNode::Match(_),
        ));
    }

    #[test]
    fn unrecognized_token_is_rejected() {
        let Err(err) = syn::parse_str::<AttributeNode>("#") else {
            panic!("expected parse error");
        };
        assert!(err.to_string().contains("expected attribute node"));
    }
}
