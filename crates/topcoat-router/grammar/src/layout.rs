use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Pat, ReturnType, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};
use topcoat_core_grammar::common::{
    ParseOption, SignatureExt, lenient,
    paths::{topcoat_context, topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro},
};

use super::common::{HandlerPath, RouteContext};

pub struct LayoutAttr {
    pub path: Option<HandlerPath>,
    /// The comma between the path and the `with(...)` entries.
    pub comma: Option<Token![,]>,
    pub context: Punctuated<RouteContext, Token![,]>,
}

impl Parse for LayoutAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.call(HandlerPath::parse_option)?;
        Ok(Self {
            comma: if path.is_some() && !input.is_empty() {
                Some(input.parse()?)
            } else {
                None
            },
            path,
            context: Punctuated::parse_terminated(input)?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for LayoutAttr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.path.pretty_print(printer);
        RouteContext::pretty_print_entries(printer, self.comma.as_ref(), &self.context);
    }
}

/// The annotated `async fn` that becomes a layout: a component taking the
/// request context as `cx` and the child content as its `slot` parameter.
pub struct LayoutItem {
    item: lenient::ItemFn,
}

impl Parse for LayoutItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: lenient::ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "layout functions must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "layout functions must declare a return type",
            ));
        }

        item.sig.cx_param()?;
        let mut params = item.sig.params();
        let slot = params.next();
        if !slot.is_some_and(|slot| matches!(&*slot.pat, Pat::Ident(pat) if pat.ident == "slot")) {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: Slot<'_>` parameter after `cx`",
            ));
        }
        if let Some(extra) = params.next() {
            return Err(syn::Error::new_spanned(
                extra,
                "layout functions only take `cx: &Cx` and `slot: Slot<'_>`",
            ));
        }
        drop(params);

        Ok(Self { item })
    }
}

pub struct Layout(LayoutAttr, LayoutItem);

impl Layout {
    #[must_use]
    pub fn new(attr: LayoutAttr, item: LayoutItem) -> Self {
        Self(attr, item)
    }

    /// Parses a layout attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`LayoutAttr`] or [`LayoutItem`], or if the item is not a valid layout
    /// function.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Layout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let ident = &item.sig.ident;

        // The component's marker is only a type, so a constant holding it
        // makes the layout's name a value for `router.layout(...)`.
        let vis = &item.vis;
        let docs = item.attrs.iter().filter(|attr| attr.path().is_ident("doc"));
        let component = quote! {
            #[#topcoat_view_macro::component]
            #item

            #(#docs)*
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            #vis const #ident: #ident = #ident { __marker: ::core::marker::PhantomData };
        };

        let render = quote! {
            fn render<'a>(
                &'a self,
                cx: &'a #topcoat_context::Cx,
                slot: #topcoat_router::Slot<'a>,
            ) -> #topcoat_view::BoxView<'a> {
                let props = <#ident as #topcoat_view::Component>::props_builder()
                    .slot(slot)
                    .build();
                ::std::boxed::Box::pin(#topcoat_view::HoistView::new(
                    #topcoat_view::internal::ThenView::new(
                        <#ident as #topcoat_view::Component>::render(#ident, cx, props)
                    )
                ))
            }
        };
        let context = RouteContext::context_method(&attr.context);
        let (layout, submit_as) =
            if let Some(path) = attr.path.as_ref().and_then(HandlerPath::absolute) {
                let layout = quote! {
                    impl #topcoat_router::Layout for #ident {
                        fn path(&self) -> &#topcoat_router::Path {
                            const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                            PATH
                        }

                        #context

                        #render
                    }
                };
                (layout, quote! { #topcoat_router::Layout })
            } else {
                let relative_path = attr
                    .path
                    .as_ref()
                    .and_then(HandlerPath::relative_path_method);
                let layout = quote! {
                    impl #topcoat_router::ModuleLayout for #ident {
                        fn module_path(&self) -> &'static str {
                            ::core::module_path!()
                        }

                        #relative_path

                        #context

                        #render
                    }
                };
                (layout, quote! { #topcoat_router::ModuleLayout })
            };

        // Discovery collects the marker erased behind its trait.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #submit_as } }
        });

        quote! {
            #component

            const _: () = {
                #layout

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<LayoutItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn attr_accepts_context_entries_with_or_without_a_path() {
        let attr: LayoutAttr = syn::parse_str("with(a)").unwrap();
        assert!(attr.path.is_none());
        assert_eq!(attr.context.len(), 1);
        let attr: LayoutAttr = syn::parse_str("\"/admin\", with(a), with(b)").unwrap();
        assert!(attr.path.is_some());
        assert_eq!(attr.context.len(), 2);
    }

    #[test]
    fn accepts_cx_and_slot() {
        syn::parse_str::<LayoutItem>(
            "async fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> { todo!() }",
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(
            parse_err("fn shell(cx: &Cx, slot: Slot<'_>) -> Result<impl View> { todo!() }")
                .contains("must be async")
        );
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(
            parse_err("async fn shell(cx: &Cx, slot: Slot<'_>) {}")
                .contains("must declare a return type")
        );
    }

    #[test]
    fn rejects_a_missing_or_misplaced_context_parameter() {
        let err =
            parse_err("async fn shell(slot: Slot<'_>, cx: &Cx) -> Result<impl View> { todo!() }");
        assert!(err.contains("cx: &Cx"));
    }

    #[test]
    fn rejects_missing_slot() {
        assert!(
            parse_err("async fn shell(cx: &Cx) -> Result<impl View> { todo!() }")
                .contains("must take a `slot: Slot<'_>` parameter")
        );
    }

    #[test]
    fn rejects_extra_parameters() {
        let err = parse_err(
            "async fn shell(cx: &Cx, slot: Slot<'_>, body: Form<A>) -> Result<impl View> { todo!() }",
        );
        assert!(err.contains("only take"));
    }
}
