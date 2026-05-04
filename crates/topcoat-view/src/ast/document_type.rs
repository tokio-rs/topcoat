use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use crate::{ast::ParseOption, output::ViewWriter};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(DOCTYPE);
    custom_keyword!(html);
}

/// The `<!DOCTYPE html>` declaration. Always renders verbatim.
pub struct DocumentType {
    pub lt_token: Token![<],
    pub exclamation_mark_token: Token![!],
    pub doctype_kw: kw::DOCTYPE,
    pub html_kw: kw::html,
    pub gt_token: Token![>],
}

impl DocumentType {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        writer.write_str_unescaped("<!DOCTYPE html>");
    }
}

impl Parse for DocumentType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            lt_token: input.parse()?,
            exclamation_mark_token: input.parse()?,
            doctype_kw: input.parse()?,
            html_kw: input.parse()?,
            gt_token: input.parse()?,
        })
    }
}

impl ParseOption for DocumentType {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![<]) && input.peek2(Token![!])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for DocumentType {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.lt_token.pretty_print(printer);
        self.exclamation_mark_token.pretty_print(printer);
        "DOCTYPE html".pretty_print(printer);
        self.gt_token.pretty_print(printer);
        printer.scan_force_break();
    }
}
