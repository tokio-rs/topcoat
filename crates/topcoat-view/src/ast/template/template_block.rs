use syn::{
    braced,
    parse::{Parse, ParseStream},
    token::Brace,
};

use crate::ast::{
    ParseOption,
    attributes::{AttributeWriter, WriteAttribute},
    view::{ViewWriter, WriteView},
};

/// A brace-delimited group of template nodes: `{ ...nodes... }`. Used as the
/// body of `if`, `for` and `match` arms, generic over the collection it
/// contains (`Nodes` or `AttributeNodes`).
pub struct TemplateBlock<T> {
    pub brace: Brace,
    pub children: T,
}

impl<T: WriteView> WriteView for TemplateBlock<T> {
    fn write(&self, writer: &mut ViewWriter) {
        self.children.write(writer);
    }
}

impl<T: WriteAttribute> WriteAttribute for TemplateBlock<T> {
    fn write(&self, writer: &mut AttributeWriter) {
        self.children.write(writer);
    }
}

impl<T: Parse> Parse for TemplateBlock<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace: braced!(content in input),
            children: content.parse()?,
        })
    }
}

impl<T: Parse> ParseOption for TemplateBlock<T> {
    fn peek(input: ParseStream) -> bool {
        input.peek(Brace)
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for TemplateBlock<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use topcoat_pretty::Delim;

        printer.move_cursor(self.brace.span().open().start());
        "{".pretty_print(printer);
        printer.move_cursor(self.brace.span().open().end());

        printer.scan_indent(1);
        printer.scan_break();

        printer.scan_trivia(false, true);
        self.children.pretty_print(printer);

        printer.move_cursor(self.brace.span().close().start());
        printer.scan_trivia(true, false);

        printer.scan_indent(-1);
        printer.scan_force_break();
        printer.scan_break();

        "}".pretty_print(printer);
        printer.move_cursor(self.brace.span().close().end());
    }
}
