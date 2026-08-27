use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{
    topcoat_context, topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro,
};

pub struct LayoutAttr {
    path: Option<LitStr>,
}

impl Parse for LayoutAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

/// The annotated `async fn` that becomes a layout: a component taking the
/// child content as its `slot` parameter and, optionally, the request
/// context as `cx`.
pub struct LayoutItem {
    item: ItemFn,
}

impl Parse for LayoutItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
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

        let (mut slot, mut cx) = (false, false);
        for arg in &item.sig.inputs {
            let FnArg::Typed(pat_type) = arg else {
                return Err(syn::Error::new_spanned(
                    arg,
                    "layout functions cannot take a `self` receiver",
                ));
            };
            let seen = match &*pat_type.pat {
                Pat::Ident(pat) if pat.ident == "slot" => &mut slot,
                Pat::Ident(pat) if pat.ident == "cx" => &mut cx,
                _ => &mut true,
            };
            if *seen {
                return Err(syn::Error::new_spanned(
                    pat_type,
                    "layout functions only accept a `slot: Slot<'_>` and an optional `cx: &Cx` parameter",
                ));
            }
            *seen = true;
        }
        if !slot {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: Slot<'_>` parameter",
            ));
        }

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

        let component = quote! {
            #[#topcoat_view_macro::component]
            #item
        };

        // The view owns copies of the request context and buffer it is
        // rendered with and drives the component's view in place, which
        // lets the view borrow them.
        let render = quote! {
            fn render<'s>(
                &'s self,
                cx: &#topcoat_context::Cx,
                buf: &#topcoat_view::ViewBuffer,
                slot: #topcoat_router::Slot<'s>,
            ) -> #topcoat_view::BoxView<'s> {
                let cx = cx.clone();
                let buf = buf.clone();
                ::std::boxed::Box::pin(#topcoat_view::internal::MoveView::new(async move {
                    let props = <#ident as #topcoat_view::Component>::props_builder()
                        .slot(slot)
                        .build();
                    let view = <#ident as #topcoat_view::Component>::render(
                        #ident, &cx, &buf, props,
                    )
                    .await?;
                    #topcoat_view::internal::drive(view).await
                }))
            }
        };
        let (layout, submit_as) = if let Some(path) = attr.path.as_ref() {
            let layout = quote! {
                impl #topcoat_router::Layout for #ident {
                    fn path(&self) -> &#topcoat_router::Path {
                        const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                        PATH
                    }

                    #render
                }
            };
            (layout, quote! { #topcoat_router::Layout })
        } else {
            let layout = quote! {
                impl #topcoat_router::ModuleLayout for #ident {
                    fn module_path(&self) -> &'static str {
                        ::core::module_path!()
                    }

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
    fn accepts_a_slot_parameter() {
        syn::parse_str::<LayoutItem>("async fn shell(slot: Result) -> Result { todo!() }").unwrap();
    }

    #[test]
    fn accepts_cx_and_slot_in_any_order() {
        syn::parse_str::<LayoutItem>("async fn shell(cx: &Cx, slot: Result) -> Result { todo!() }")
            .unwrap();
        syn::parse_str::<LayoutItem>("async fn shell(slot: Result, cx: &Cx) -> Result { todo!() }")
            .unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(
            parse_err("fn shell(slot: Result) -> Result { todo!() }").contains("must be async")
        );
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(
            parse_err("async fn shell(slot: Result) {}").contains("must declare a return type")
        );
    }

    #[test]
    fn rejects_missing_slot() {
        assert!(
            parse_err("async fn shell(cx: &Cx) -> Result { todo!() }")
                .contains("must take a `slot: Result` parameter")
        );
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn shell(&self, slot: Result) -> Result { todo!() }");
        assert!(err.contains("cannot take a `self` receiver"));
    }

    #[test]
    fn rejects_unknown_parameter_names() {
        let err = parse_err("async fn shell(slot: Result, body: Form<A>) -> Result { todo!() }");
        assert!(err.contains("only accept"));
    }

    #[test]
    fn rejects_duplicate_slot_parameters() {
        let err = parse_err("async fn shell(slot: Result, slot: Result) -> Result { todo!() }");
        assert!(err.contains("only accept"));
    }
}
