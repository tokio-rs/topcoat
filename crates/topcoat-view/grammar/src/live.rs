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

/// The parsed body of a `live!` invocation: an optional `cx =>` argument and
/// the statements of an async block.
///
/// Expands to a live region view. The region's id is derived from the
/// identity of the context and the body's source location, so it is the same
/// across renders.
pub struct Live {
    /// The context named by a leading `cx =>` argument. The region then holds
    /// its own clone of it. [`None`] uses the context of the enclosing body.
    pub cx: Option<LeadingCx>,
    /// The statements of the region's body, which run inside an `async move`
    /// block.
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
        // Read the input span: line!() and column!() would name the outer
        // view! invocation and merge its distinct live! sites.
        let start = self
            .body
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
        let body = &self.body;
        let bind_cx = self.cx.as_ref().map(|cx| {
            let cx = &cx.cx;
            quote! { let __cx: #topcoat_context::Cx = (#cx).clone(); }
        });

        quote! {{
            #bind_cx
            let __region = #topcoat_view::RegionId::new(#topcoat_context::identity(&__cx), #site);
            #topcoat_view::internal::LiveView::new(
                __region,
                async move {
                    let __cx: &#topcoat_context::Cx = &__cx;
                    #(#body)*
                },
            )
        }}
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    /// Keeps statements on separate lines, preserving comments and blank lines.
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.cx.pretty_print(printer);
        for (index, stmt) in self.body.iter().enumerate() {
            stmt.pretty_print(printer);
            if index < self.body.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_break();
                printer.scan_trivia(true, true);
            }
        }
    }
}

/// The parsed body of an `emit!` invocation, written with the same syntax as
/// `view!`.
///
/// Expands to an expression that renders the view into the enclosing `live!`
/// region and awaits it. The expression evaluates to a `Result` holding an
/// `EmitToken`, or to the error the view failed to render with.
pub struct Emit {
    /// The markup to emit, with an optional `cx =>` argument.
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
            #topcoat_view::internal::LiveView::drive(__region, #view).await
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
        assert!(tokens.starts_with(":: topcoat_view :: internal :: LiveView :: drive (__region ,"));
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
        assert!(tokens.contains("Cx = & __cx ;"), "{tokens}");
    }

    #[test]
    fn a_live_body_is_wrapped_in_an_async_block() {
        let tokens = syn::parse_str::<Live>("let x = 1; emit! { <div></div> }")
            .unwrap()
            .to_token_stream()
            .to_string();
        assert!(tokens.contains("async move {"), "{tokens}");
        assert!(tokens.contains("let x = 1 ;"), "{tokens}");
    }
}
