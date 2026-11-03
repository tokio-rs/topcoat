use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::common::paths::{topcoat_context, topcoat_core, topcoat_view};

use crate::view::{
    View,
    hir::{LowerView, ViewBuilder},
};

pub struct Live {
    pub body: TokenStream,
}

impl Parse for Live {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            body: input.parse()?,
        })
    }
}

impl ToTokens for Live {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // Read the input span: line!() and column!() would name the outer
        // view! invocation and merge its distinct live! sites.
        let start = self.body.span().start();
        let line = u32::try_from(start.line).expect("source line fits in u32");
        let column = u32::try_from(start.column + 1).expect("source column fits in u32");
        let site = quote! {
            const {
                #topcoat_core::identity::SiteKey::new(::core::file!(), #line, #column, 0)
            }
        };
        let body = &self.body;
        quote! {{
            let __region = #topcoat_view::RegionId::new(
                #topcoat_context::identity(&cx),
                #site,
            );
            #topcoat_view::internal::LiveView::new(__region, async move { #body })
        }}
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for Live {
    /// Keeps statements on separate lines, preserving comments and blank lines.
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use syn::parse::Parser;

        let Ok(body) = syn::Block::parse_within.parse2(self.body.clone()) else {
            syn::Expr::Verbatim(self.body.clone()).pretty_print(printer);
            return;
        };
        for (index, stmt) in body.iter().enumerate() {
            stmt.pretty_print(printer);
            if index < body.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_break();
                printer.scan_trivia(true, true);
            }
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
        let view = builder.finish().emit_emit();

        // The view borrows the context, so the binding has to outlive the
        // drive it is emitted for.
        quote! {{
            let cx: &#topcoat_context::Cx = &cx;
            #topcoat_view::internal::LiveView::drive(__region, #view).await
        }}
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for Emit {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
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
        assert!(
            tokens.contains(":: topcoat_view :: internal :: LiveView :: drive (__region ,"),
            "{tokens}"
        );
        assert!(tokens.ends_with(". await }"), "{tokens}");
    }

    #[test]
    fn an_emitted_view_is_self_contained() {
        let tokens = emit("<div></div>");
        assert!(tokens.contains("ScopeView :: self_contained ("), "{tokens}");
    }

    #[test]
    fn an_emitted_view_borrows_the_call_site_cx_without_cloning_it() {
        let tokens = emit("<div></div>");
        let binding = tokens.find("Cx = & cx ;").expect(&tokens);
        let drive = tokens.find("LiveView :: drive (").expect(&tokens);
        assert!(binding < drive, "{tokens}");
        assert!(!tokens.contains("clone ()"), "{tokens}");
    }

    #[test]
    fn a_live_region_is_identified_by_the_call_site_cx() {
        let tokens = syn::parse_str::<Live>("emit! { <div></div> }")
            .unwrap()
            .to_token_stream()
            .to_string();
        let identity = tokens.find("identity (& cx)").expect(&tokens);
        let body = tokens.find("async move {").expect(&tokens);
        assert!(identity < body, "{tokens}");
        assert!(!tokens.contains("clone ()"), "{tokens}");
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

    #[test]
    fn incomplete_live_bodies_are_forwarded() {
        for source in [
            "let x = value.;",
            "let x = call(,);",
            "let x = module::;",
            "if ready { let x = ; }",
            "let f = |value| value.;",
            "match value { Some(x) => x., None => () }",
        ] {
            let live = syn::parse_str::<Live>(source).unwrap();
            let original: TokenStream = source.parse().unwrap();
            assert_eq!(live.body.to_string(), original.to_string());
            let expanded = live.to_token_stream().to_string();
            assert!(expanded.contains(&original.to_string()), "{expanded}");
            assert!(expanded.contains("async move {"), "{expanded}");
        }
    }

    #[test]
    fn live_body_preserves_its_source_location() {
        let live = syn::parse_str::<Live>("\n    let x = value.;").unwrap();
        assert_eq!(
            live.body.span().start(),
            proc_macro2::LineColumn { line: 2, column: 4 },
        );
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn incomplete_live_body_formats_verbatim() {
        use topcoat_core_grammar::common::pretty::{Registry, pretty_print_str};

        let registry = Registry::one::<Live>("live");
        let body = "let value = source; // keep this comment\n    let next = value.;";
        let source = format!("live! {{ {body} }}");
        let formatted = pretty_print_str(&registry, &source).unwrap();
        assert!(formatted.contains(body), "{formatted}");
        assert_eq!(pretty_print_str(&registry, &formatted).unwrap(), formatted);
    }
}
