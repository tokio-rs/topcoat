use syn::{
    Path, Token, bracketed,
    parse::{Parse, ParseStream},
    token::Bracket,
};

use crate::ast::{Attributes, ParseOption};

/// A component's opening tag: `[path attr=value ...]`.
pub struct ComponentOpeningTag {
    pub bracket_token: Bracket,
    pub path: Path,
    pub attributes: Attributes,
}

impl Parse for ComponentOpeningTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: bracketed!(content in input),
            path: content.parse()?,
            attributes: content.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ComponentOpeningTag {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        printer.scan_begin(topcoat_pretty::BreakMode::Consistent);
        "[".pretty_print(printer);
        self.path.pretty_print(printer);
        if !self.attributes.is_empty() {
            printer.scan_break();
            printer.scan_indent(1);
            self.attributes.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        "]".pretty_print(printer);
        printer.scan_end();
    }
}

/// A self-closing component tag: `[path attr=value /]`.
pub struct ComponentSelfClosingTag {
    pub bracket_token: Bracket,
    pub path: Path,
    pub attributes: Attributes,
    pub slash: Token![/],
}

impl Parse for ComponentSelfClosingTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: bracketed!(content in input),
            path: content.parse()?,
            attributes: content.parse()?,
            slash: content.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ComponentSelfClosingTag {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        "[".pretty_print(printer);
        self.path.pretty_print(printer);
        if !self.attributes.is_empty() {
            printer.scan_break();
            printer.scan_indent(1);
            self.attributes.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_break();
        }
        " /]".pretty_print(printer);
    }
}

/// A component's closing tag: `[/path]`.
pub struct ComponentClosingTag {
    pub bracket_token: Bracket,
    pub slash: Token![/],
    pub path: Path,
}

impl Parse for ComponentClosingTag {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: bracketed!(content in input),
            slash: content.parse()?,
            path: content.parse()?,
        })
    }
}

impl ParseOption for ComponentClosingTag {
    fn peek(input: ParseStream) -> bool {
        fn inner(input: ParseStream) -> syn::Result<()> {
            let content;
            let _ = bracketed!(content in input.fork());
            let _: Token![/] = content.parse()?;
            Ok(())
        }

        inner(input).is_ok()
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ComponentClosingTag {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        "[/".pretty_print(printer);
        self.path.pretty_print(printer);
        "]".pretty_print(printer);
    }
}
