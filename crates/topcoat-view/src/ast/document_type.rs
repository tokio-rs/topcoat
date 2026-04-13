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

pub struct DocumentType {
    pub lt_token: Token![<],
    pub exclamation_mark_token: Token![!],
    pub doctype_kw: kw::DOCTYPE,
    pub html_kw: kw::html,
    pub gt_token: Token![>],
}

impl DocumentType {
    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        writer.push_str("<!DOCTYPE html>");
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
