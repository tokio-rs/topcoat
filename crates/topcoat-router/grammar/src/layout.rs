use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat, ReturnType,
    parse::{Parse, ParseStream},
    parse_quote,
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

/// A layout function parameter, classified by name.
enum LayoutArg {
    /// The `cx: &Cx` request context parameter.
    Cx,
    /// The `slot: Result` child content parameter.
    Slot,
}

pub struct LayoutItem {
    item: ItemFn,
    args: Vec<LayoutArg>,
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

        let mut args: Vec<LayoutArg> = Vec::new();
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(receiver) => {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "layout functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi)
                        if pi.ident == "slot"
                            && !args.iter().any(|arg| matches!(arg, LayoutArg::Slot)) =>
                    {
                        args.push(LayoutArg::Slot);
                    }
                    Pat::Ident(pi)
                        if pi.ident == "cx"
                            && !args.iter().any(|arg| matches!(arg, LayoutArg::Cx)) =>
                    {
                        args.push(LayoutArg::Cx);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "layout functions only accept a `slot: Result` and an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        if !args.iter().any(|arg| matches!(arg, LayoutArg::Slot)) {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: Result` parameter",
            ));
        }

        Ok(Self { item, args })
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
        let args = &self.1.args;
        let ident = &item.sig.ident;

        let mut face = item.clone();
        for (arg, input) in args.iter().zip(&mut face.sig.inputs) {
            if let (LayoutArg::Slot, FnArg::Typed(pat_type)) = (arg, input) {
                pat_type.ty = parse_quote! { #topcoat_router::Slot<'_> };
            }
        }
        let marker = quote! {
            #[#topcoat_view_macro::component]
            #face
        };

        // The view picks up the request context it is first polled with, so
        // an outer layout's derived context reaches this one. It owns that
        // context and drives the component's view in place, which lets the
        // view borrow it.
        let render = quote! {
            fn render<'s>(
                &'s self,
                slot: #topcoat_router::Slot<'s>,
            ) -> #topcoat_view::BoxView<'s> {
                ::std::boxed::Box::pin(#topcoat_view::internal::LazyView::new(
                    move |cx: #topcoat_context::Cx| {
                        #topcoat_view::internal::MoveView::new(async move {
                            let props = <#ident as #topcoat_view::Component>::props_builder()
                                .slot(slot)
                                .build();
                            let view = <#ident as #topcoat_view::Component>::render(
                                #ident, &cx, props,
                            )
                            .await?;
                            <#topcoat_view::internal::MoveView>::drive(&cx, view).await
                        })
                    },
                ))
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
            #marker

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
