use quote::quote;
use syn::{
    Ident, Path, Token, Type, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
};

use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_runtime;

use crate::view::{ExprKind, ViewWriter, WriteView};

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

        // Bound to a local because it is interpolated inside the `#(...)*`
        // repetition below, where a bare `#topcoat_runtime` would expand to a
        // `let` binding that cannot shadow the imported constant.
        let read_signal = quote!(#topcoat_runtime::ReadSignal);

        writer.write_expr(
            ExprKind::Node,
            quote! {
                #topcoat_runtime::ReactiveScope::new(
                    __cx,
                    (#(#read_signal::new(&#signals),)*),
                    #path,
                ).await?
            },
        );
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
impl topcoat_core_grammar::pretty::PrettyPrint for ReactiveScope {
    fn pretty_print(&self, _printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        todo!();
    }
}
