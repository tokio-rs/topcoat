use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_view};

use crate::view::{
    View,
    hir::{LowerView, ViewBuilder},
};

pub struct Live {
    pub body: Vec<syn::Stmt>,
}

impl Parse for Live {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            body: input.call(syn::Block::parse_within)?,
        })
    }
}

impl ToTokens for Live {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let body = &self.body;
        quote! {
            #topcoat_view::internal::LiveView::new(async move {
                #(#body)*
            })
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        for (index, stmt) in self.body.iter().enumerate() {
            stmt.pretty_print(printer);
            if index < self.body.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
        if self.body.len() > 1 {
            printer.scan_force_break();
        }
    }
}

pub struct Emit {
    pub view: View,
}

impl Parse for Emit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            view: input.parse()?,
        })
    }
}

impl ToTokens for Emit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut builder = ViewBuilder::new();
        self.view.nodes.lower(&mut builder);
        let owns_cx = self.view.cx.is_some();
        let view = builder.finish().emit_emit(owns_cx);

        let view = match &self.view.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #view
                }}
            }
            None => view,
        };

        quote! {
            #topcoat_view::internal::LiveView::drive(#view).await
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Emit {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.view.pretty_print(printer);
    }
}
