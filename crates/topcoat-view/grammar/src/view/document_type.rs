use syn::{
    Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::view::hir::{LowerView, ViewBuilder};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(DOCTYPE);
    custom_keyword!(html);
}

/// The `<!DOCTYPE html>` declaration. It always renders as written.
pub struct DocumentType {
    /// The `<`.
    pub lt_token: Token![<],
    /// The `!`.
    pub exclamation_mark_token: Token![!],
    /// The `DOCTYPE` keyword.
    pub doctype_kw: kw::DOCTYPE,
    /// The `html` keyword.
    pub html_kw: kw::html,
    /// The `>`.
    pub gt_token: Token![>],
}

impl LowerView for DocumentType {
    fn lower(&self, builder: &mut ViewBuilder) {
        builder.str_unescaped("<!DOCTYPE html>");
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
impl topcoat_core_grammar::pretty::PrettyPrint for DocumentType {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.lt_token.pretty_print(printer);
        self.exclamation_mark_token.pretty_print(printer);
        "DOCTYPE html".pretty_print(printer);
        self.gt_token.pretty_print(printer);
        printer.scan_force_break();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<DocumentType>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_doctype_html() {
        syn::parse_str::<DocumentType>("<!DOCTYPE html>").unwrap();
    }

    #[test]
    fn rejects_lowercase_doctype() {
        // The HTML5 spec is case-insensitive, but the macro only accepts the
        // canonical uppercase spelling.
        assert!(parse_err("<!doctype html>").contains("expected `DOCTYPE`"));
    }

    #[test]
    fn rejects_other_doctype_target() {
        assert!(parse_err("<!DOCTYPE xml>").contains("expected `html`"));
    }
}
