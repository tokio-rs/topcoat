use quote::quote;
use syn::{
    Ident, Path, Token, Type, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};

use crate::ast::{
    ParseOption,
    view::{ViewWriter, WriteView},
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(track);
}

pub struct ReactiveScope {
    pub track_kw: kw::track,
    pub path: Path,
    pub paren_token: Paren,
    pub signals: Punctuated<Ident, Token![,]>,
}

pub struct SignalParam {
    pub ident: Ident,
    pub colon_token: Token![:],
    pub ty: Type,
}

impl Parse for SignalParam {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl WriteView for ReactiveScope {
    fn write(&self, writer: &mut ViewWriter) {
        let path = &self.path;
        let signals = self.signals.iter();

        writer.write_expr(quote! {
            ::topcoat::runtime::ReactiveScope::new(
                __cx,
                (#(::topcoat::runtime::ReadSignal::new(&#signals),)*),
                #path,
            ).await?
        });
    }
}

impl Parse for ReactiveScope {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            track_kw: input.parse()?,
            path: input.parse()?,
            paren_token: parenthesized!(content in input),
            signals: Punctuated::parse_terminated(&content)?,
        })
    }
}

impl ParseOption for ReactiveScope {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::track)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ReactiveScope {
    fn pretty_print(&self, _printer: &mut topcoat_pretty::Printer<'_>) {
        todo!();
    }
}
