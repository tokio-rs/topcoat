use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat, ReturnType, Visibility,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{
    topcoat_context, topcoat_error, topcoat_inventory, topcoat_router, topcoat_view,
    topcoat_view_macro,
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
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let output = &item.sig.output;

        // Component face: wraps a child view inline from `view!`. It always
        // takes `cx` (feeding the handler's injected context parameter), and
        // the child is already rendered, so it is passed as the `slot` prop
        // and handed to the handler as an `Ok` result. The marker struct this
        // expands to is a unit struct, so `#ident` stays a value usable
        // directly in `router.layout(...)`.
        let component_args = args.iter().map(|arg| match arg {
            LayoutArg::Cx => quote! { cx },
            LayoutArg::Slot => quote! { slot },
        });
        quote! {
            #[#topcoat_view_macro::component]
            #vis async fn #ident(cx: &#topcoat_context::Cx, slot: #topcoat_error::Result<#topcoat_view::View>) #output {
                #ident::handler(cx #(, #component_args)*).await
            }
        }
        .to_tokens(tokens);

        // The user's real body, attached to the marker as an associated
        // function. Associated items are reached through the type rather than
        // lexical scope, so hiding the impl inside the anonymous const below
        // keeps the module namespace clean while both the component face and
        // the render function can call `#ident::handler`. The injected `__cx`
        // parameter carries the ambient context that `view!` bodies read.
        let mut handler = item.clone();
        handler.sig.ident = format_ident!("handler", span = ident.span());
        handler.vis = Visibility::Inherited;
        handler
            .sig
            .generics
            .params
            .insert(0, parse_quote! { '__cx });
        handler
            .sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx #topcoat_context::Cx });
        handler
            .attrs
            .push(parse_quote! { #[allow(clippy::unused_async)] });

        // The render function backing the registered layout passes the
        // already-rendered slot result through untouched, so the layout body
        // wraps the inner page's output.
        let render_args = args.iter().map(|arg| match arg {
            LayoutArg::Cx => quote! { cx },
            LayoutArg::Slot => quote! { slot },
        });
        let render = quote! {
            |cx, slot| ::std::boxed::Box::pin(#ident::handler(cx #(, #render_args)*))
        };

        // The erased layout is built once in a `const` so it can be used from
        // both the `From` impl (backing manual `router.layout(#ident)`
        // registration) and the discovery submission (which expands to a
        // `static`, requiring a const initializer).
        let erased = if let Some(path) = attr.path.as_ref() {
            quote! {
                const ERASED: #topcoat_router::LayoutFn = #topcoat_router::LayoutFn::new(
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );

                impl ::core::convert::From<#ident> for #topcoat_router::LayoutFn {
                    fn from(_: #ident) -> Self {
                        ERASED
                    }
                }
            }
        } else {
            quote! {
                const ERASED: #topcoat_router::ModuleLayoutFn =
                    #topcoat_router::ModuleLayoutFn::new(module_path!(), #render);

                impl ::core::convert::From<#ident> for #topcoat_router::ModuleLayoutFn {
                    fn from(_: #ident) -> Self {
                        ERASED
                    }
                }
            }
        };

        let submit =
            cfg!(feature = "discover").then(|| quote! { #topcoat_inventory::submit! { ERASED } });

        quote! {
            const _: () = {
                impl #ident {
                    #handler
                }

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
