use std::sync::atomic::Ordering;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Block, Ident, Token, Type, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Brace,
};
use uuid::Uuid;

use crate::ast::{
    ParseOption,
    view::{Node, TemplateBlock, ViewWriter, WriteView},
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(track);
}

/// A `track |signal1: T1, signal2: T2| { ...rust body... }` expression that
/// re-evaluates its body whenever any of the listed signals changes. Resembles
/// a closure with a leading `track` keyword; each parameter requires an explicit
/// type annotation.
pub struct TrackedBlock {
    pub track_kw: kw::track,
    pub or1_token: Token![|],
    pub signals: Punctuated<SignalParam, Token![,]>,
    pub or2_token: Token![|],
    pub brace: Brace,
    pub body: TokenStream,
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

impl WriteView for TrackedBlock {
    fn write(&self, writer: &mut ViewWriter) {
        let signal_params = self.signals.iter().map(|s| {
            let ident = &s.ident;
            let ty = &s.ty;
            quote! { #ident: #ty }
        });
        let body = &self.body;

        static AUTO_INCREMENT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let increment = AUTO_INCREMENT.fetch_add(1, Ordering::Relaxed);

        let render_fn_ident = format_ident!("__track_render_{increment}");
        let route_fn_ident = format_ident!("__track_route_{increment}");

        let route = format!("/_topcoat/partials/{}", Uuid::new_v4());

        writer.statement(quote! {
            async fn #render_fn_ident(
                cx: &::topcoat::context::Cx,
                #(#signal_params),*
            ) -> ::topcoat::router::Result {
                view! { #body }
            }

            #[::topcoat::router::route(GET #route)]
            async fn #route_fn_ident(cx: &::topcoat::context::Cx) {

            }
        });
    }
}

impl Parse for TrackedBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            track_kw: input.parse()?,
            or1_token: input.parse()?,
            signals: {
                let mut signals = Punctuated::new();
                while !input.peek(Token![|]) {
                    signals.push_value(input.parse()?);
                    if input.peek(Token![|]) {
                        break;
                    }
                    signals.push_punct(input.parse()?);
                }
                signals
            },
            or2_token: input.parse()?,
            brace: braced!(content in input),
            body: content.parse()?,
        })
    }
}

impl ParseOption for TrackedBlock {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::track)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TrackedBlock {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        todo!();
    }
}
