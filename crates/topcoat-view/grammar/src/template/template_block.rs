use syn::{
    braced,
    parse::{Parse, ParseStream},
    token::Brace,
};

use topcoat_core_grammar::ParseOption;

use crate::{
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
impl<T: topcoat_core_grammar::pretty::PrettyPrint> topcoat_core_grammar::pretty::PrettyPrint
    for TemplateBlock<T>
{
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::Delim;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::Nodes;

    fn parse(source: &str) -> TemplateBlock<Nodes> {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_empty_block() {
        assert!(parse("{}").children.is_empty());
    }

    #[test]
    fn parses_single_child() {
        let block = parse(r#"{ "hi" }"#);
        assert_eq!(block.children.len(), 1);
    }

    #[test]
    fn parses_multiple_children() {
        let block = parse(r#"{ "a" "b" "c" }"#);
        assert_eq!(block.children.len(), 3);
    }

    #[test]
    fn requires_braces() {
        assert!(syn::parse_str::<TemplateBlock<Nodes>>(r#""hi""#).is_err());
    }
}
