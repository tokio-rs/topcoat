use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_context, topcoat_core, topcoat_view},
};

use crate::{
    leading_cx::LeadingCx,
    view::{
        View,
        hir::{LowerView, ViewBuilder},
    },
};

pub struct Live {
    pub cx: Option<LeadingCx>,
    pub body: Vec<syn::Stmt>,
}

impl Parse for Live {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cx: input.call(LeadingCx::parse_option)?,
            body: input.call(syn::Block::parse_within)?,
        })
    }
}

impl ToTokens for Live {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let body = &self.body;
        // Read the input span: line!() and column!() would name the outer
        // view! invocation and merge its distinct live! sites.
        let start = body
            .first()
            .map_or_else(Span::call_site, Spanned::span)
            .start();
        let line = u32::try_from(start.line).expect("source line fits in u32");
        let column = u32::try_from(start.column + 1).expect("source column fits in u32");
        let site = quote! {
            const {
                #topcoat_core::identity::SiteKey::new(::core::file!(), #line, #column, 0)
            }
        };
        let borrow_cx = self.cx.as_ref().map(|_| quote! { let __cx = &__cx; });
        let cx = if self.cx.is_some() {
            quote! { &__cx }
        } else {
            quote! { __cx }
        };
        let view = quote! {
            #topcoat_view::internal::LiveView::new(
                #topcoat_context::identity(#cx),
                #site,
                #topcoat_view::pass(#cx),
                async move {
                    #borrow_cx
                    #(#body)*
                },
            )
        };
        match &self.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #view
                }}
                .to_tokens(tokens);
            }
            None => view.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        if let Some(cx) = &self.cx {
            cx.pretty_print(printer);
        }
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

        let drive = quote! {
            #topcoat_view::internal::LiveView::drive(#view).await
        };

        // The view borrows the context rather than moving it, so the binding
        // has to outlive the drive it is emitted for.
        match &self.view.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #drive
                }}
            }
            None => drive,
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

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    fn emit(source: &str) -> String {
        syn::parse_str::<Emit>(source)
            .unwrap()
            .to_token_stream()
            .to_string()
    }

    #[test]
    fn an_emitted_view_is_driven_into_the_live_view() {
        let tokens = emit("<div></div>");
        assert!(tokens.starts_with(":: topcoat_view :: internal :: LiveView :: drive ("));
        assert!(tokens.ends_with(". await"), "{tokens}");
    }

    #[test]
    fn an_emitted_view_is_self_contained() {
        let tokens = emit("<div></div>");
        assert!(tokens.contains("ScopeView :: self_contained ("), "{tokens}");
    }

    #[test]
    fn an_emitted_view_keeps_the_ambient_cx() {
        let tokens = emit("<div></div>");
        assert!(!tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn an_explicit_cx_binds_the_context_identifier() {
        let tokens = emit("cx => <div></div>");
        assert!(tokens.contains("Cx = (cx) . clone () ;"), "{tokens}");
        assert!(tokens.contains("let __cx = & __cx ;"), "{tokens}");
    }

    #[test]
    fn a_live_body_is_wrapped_in_an_async_block() {
        let tokens = syn::parse_str::<Live>("let x = 1; emit! { <div></div> }")
            .unwrap()
            .to_token_stream()
            .to_string();
        assert!(tokens.contains("async move { let x = 1 ;"), "{tokens}");
    }
}
