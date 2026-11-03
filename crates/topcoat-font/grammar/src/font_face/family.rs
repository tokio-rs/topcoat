use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::common::{ParseOption, lenient};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(family);
}

pub struct FontFamily {
    pub key: FontFamilyKey,
    pub colon_token: Token![:],
    pub value: FontFamilyValue,
}

impl Parse for FontFamily {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontFamily {
    fn peek(input: ParseStream) -> bool {
        FontFamilyKey::peek(input)
    }
}

impl ToTokens for FontFamily {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontFamily {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct FontFamilyKey {
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub family_kw: kw::family,
}

impl Parse for FontFamilyKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            font_kw: input.parse()?,
            dash_token: input.parse()?,
            family_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontFamilyKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::font) && input.peek2(Token![-]) && input.peek3(kw::family)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontFamilyKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.font_kw.span().start());
        "font-family".pretty_print(printer);
        printer.move_cursor(self.family_kw.span().end());
    }
}

pub struct FontFamilyValue(pub lenient::Expr);

impl Parse for FontFamilyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for FontFamilyValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontFamilyValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}
