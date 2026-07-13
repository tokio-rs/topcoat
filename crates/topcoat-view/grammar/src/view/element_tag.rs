use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;

use crate::{attributes::Attributes, view::ElementName};

/// An element's opening tag: `<name attr=value ...>`.
pub struct OpeningTag {
    pub lt: Token![<],
    pub name: ElementName,
    pub attributes: Attributes,
    pub gt: Token![>],
}

impl Parse for OpeningTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            lt: input.parse()?,
            name: input.parse()?,
            attributes: input.parse()?,
            gt: input.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for OpeningTag {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        printer.scan_begin(topcoat_core_grammar::pretty::BreakMode::Consistent);
        self.lt.pretty_print(printer);
        self.name.pretty_print(printer);
        if !self.attributes.is_empty() {
            printer.scan_indent(1);
            printer.scan_break();
            " ".pretty_print(printer);
            self.attributes.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        self.gt.pretty_print(printer);
        printer.scan_end();
    }
}

/// An element's self-closing tag: `<name attr=value ... />`.
pub struct SelfClosingTag {
    pub lt: Token![<],
    pub name: ElementName,
    pub attributes: Attributes,
    pub slash: Token![/],
    pub gt: Token![>],
}

impl Parse for SelfClosingTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            lt: input.parse()?,
            name: input.parse()?,
            attributes: input.parse()?,
            slash: input.parse()?,
            gt: input.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for SelfClosingTag {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        printer.scan_begin(topcoat_core_grammar::pretty::BreakMode::Consistent);
        self.lt.pretty_print(printer);
        self.name.pretty_print(printer);
        if !self.attributes.is_empty() {
            printer.scan_indent(1);
            printer.scan_break();
            " ".pretty_print(printer);
            self.attributes.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        " ".pretty_print(printer);
        self.slash.pretty_print(printer);
        self.gt.pretty_print(printer);
        printer.scan_end();
    }
}

/// The tag an element starts with: either an opening tag or a self-closing
/// tag.
pub enum ElementTag {
    Opening(OpeningTag),
    SelfClosing(SelfClosingTag),
}

impl Parse for ElementTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lt = input.parse()?;
        let name = input.parse()?;
        let attributes = input.parse()?;
        if input.peek(Token![/]) {
            Ok(Self::SelfClosing(SelfClosingTag {
                lt,
                name,
                attributes,
                slash: input.parse()?,
                gt: input.parse()?,
            }))
        } else {
            Ok(Self::Opening(OpeningTag {
                lt,
                name,
                attributes,
                gt: input.parse()?,
            }))
        }
    }
}

/// An element's closing tag: `</name>`.
pub struct ClosingTag {
    pub lt: Token![<],
    pub slash: Token![/],
    pub name: ElementName,
    pub gt: Token![>],
}

impl Parse for ClosingTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            lt: input.parse()?,
            slash: input.parse()?,
            name: input.parse()?,
            gt: input.parse()?,
        })
    }
}

impl ParseOption for ClosingTag {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![<]) && input.peek2(Token![/])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for ClosingTag {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.lt.pretty_print(printer);
        self.slash.pretty_print(printer);
        self.name.pretty_print(printer);
        self.gt.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_opening_tag_without_attributes() {
        let tag: OpeningTag = syn::parse_str("<div>").unwrap();
        assert_eq!(tag.name.string_name().as_deref(), Some("div"));
        assert!(tag.attributes.is_empty());
    }

    #[test]
    fn parses_opening_tag_with_attributes() {
        let tag: OpeningTag = syn::parse_str(r#"<a href="/" class="link">"#).unwrap();
        assert_eq!(tag.name.string_name().as_deref(), Some("a"));
        assert_eq!(tag.attributes.items.len(), 2);
    }

    #[test]
    fn parses_self_closing_tag() {
        let tag: SelfClosingTag = syn::parse_str(r#"<path d="M0 0h24v24H0z"/>"#).unwrap();
        assert_eq!(tag.name.string_name().as_deref(), Some("path"));
        assert_eq!(tag.attributes.items.len(), 1);
    }

    #[test]
    fn element_tag_distinguishes_opening_and_self_closing() {
        let tag: ElementTag = syn::parse_str("<div>").unwrap();
        assert!(matches!(tag, ElementTag::Opening(_)));

        let tag: ElementTag = syn::parse_str("<circle/>").unwrap();
        assert!(matches!(tag, ElementTag::SelfClosing(_)));
    }

    #[test]
    fn parses_closing_tag() {
        let tag: ClosingTag = syn::parse_str("</div>").unwrap();
        assert_eq!(tag.name.string_name().as_deref(), Some("div"));
    }

    #[test]
    fn closing_tag_rejects_opening_tag() {
        // `<div>` is an opening tag, not a closing one.
        assert!(syn::parse_str::<ClosingTag>("<div>").is_err());
    }
}
