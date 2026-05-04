use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use crate::ast::{Attributes, ElementName, ParseOption};

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
impl topcoat_pretty::PrettyPrint for OpeningTag {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        printer.scan_begin(topcoat_pretty::BreakMode::Consistent);
        self.lt.pretty_print(printer);
        self.name.pretty_print(printer);
        if !self.attributes.is_empty() {
            printer.scan_break();
            printer.scan_indent(1);
            self.attributes.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        self.gt.pretty_print(printer);
        printer.scan_end();
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
impl topcoat_pretty::PrettyPrint for ClosingTag {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.lt.pretty_print(printer);
        self.slash.pretty_print(printer);
        self.name.pretty_print(printer);
        self.gt.pretty_print(printer);
    }
}
