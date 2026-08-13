use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{
    topcoat_inventory, topcoat_router, topcoat_view, topcoat_view_macro,
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

        let mut has_slot = false;
        let mut has_cx = false;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(receiver) => {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "layout functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) if pi.ident == "slot" && !has_slot => {
                        has_slot = true;
                    }
                    Pat::Ident(pi) if pi.ident == "cx" && !has_cx => {
                        has_cx = true;
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "layout functions only accept a `slot: ViewHandle<'_>` \
                             and an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        if !has_slot {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: ViewHandle<'_>` parameter",
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
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let inputs = &item.sig.inputs;
        let output = &item.sig.output;

        // Component face: the user's body becomes the component body with
        // its parameters as declared, so its tail `view!` compiles onto the
        // live render. The `slot` prop is the handle of the wrapped content,
        // spliced to bubble its errors or consumed with `live match` to
        // catch them. The marker struct this expands to is a unit struct, so
        // `#ident` stays a value usable directly in `router.layout(...)`.
        let stmts = &item.block.stmts;
        let attrs = &item.attrs;
        quote! {
            #(#attrs)*
            #[#topcoat_view_macro::component]
            #vis async fn #ident(#inputs) #output {
                #(#stmts)*
            }
        }
        .to_tokens(tokens);

        // The render function backing the registered layout hands the
        // framework-minted handle of the inner content to the component as
        // its `slot` prop.
        let render = quote! {
            |cx, slot, fill| ::std::boxed::Box::pin(async move {
                use #topcoat_view::Component;
                let props = #ident::props_builder().slot(slot).build();
                #[allow(clippy::default_constructed_unit_structs)]
                Component::render(#ident::default(), cx, props, fill).await
            })
        };

        // The erased layout is built once in a `const` so it can be used from
        // both the `From` impl (backing manual `router.layout(#ident)`
        // registration) and the discovery submission (which expands to a
        // `static`, requiring a const initializer). It is named after the
        // layout so the render closure carries that name in backtraces and
        // profiles.
        let erased = if let Some(path) = attr.path.as_ref() {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::LayoutFn = #topcoat_router::LayoutFn::new(
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );

                impl ::core::convert::From<#ident> for #topcoat_router::LayoutFn {
                    fn from(_: #ident) -> Self {
                        #ident
                    }
                }
            }
        } else {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::ModuleLayoutFn =
                    #topcoat_router::ModuleLayoutFn::new(module_path!(), #render);

                impl ::core::convert::From<#ident> for #topcoat_router::ModuleLayoutFn {
                    fn from(_: #ident) -> Self {
                        #ident
                    }
                }
            }
        };

        let submit =
            cfg!(feature = "discover").then(|| quote! { #topcoat_inventory::submit! { #ident } });

        quote! {
            const _: () = {
                #erased

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
                .contains("must take a `slot: ViewHandle")
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
