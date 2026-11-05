use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Token, Visibility,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
};
use topcoat_core_grammar::common::{
    ParseOption, SignatureExt, lenient,
    paths::{topcoat_context, topcoat_inventory, topcoat_router},
};

use super::common::{HandlerPath, RouteContext};

pub struct LayerAttr {
    pub path: Option<HandlerPath>,
    /// The comma between the path and the `with(...)` entries.
    pub comma: Option<Token![,]>,
    pub context: Punctuated<RouteContext, Token![,]>,
}

impl Parse for LayerAttr {
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
impl topcoat_core_grammar::common::pretty::PrettyPrint for LayerAttr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.path.pretty_print(printer);
        RouteContext::pretty_print_entries(printer, self.comma.as_ref(), &self.context);
    }
}

pub struct LayerItem {
    item: lenient::ItemFn,
}

impl Parse for LayerItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: lenient::ItemFn = input.parse()?;
        item.sig.cx_param()?;
        Ok(Self { item })
    }
}

pub struct Layer(LayerAttr, LayerItem);

impl Layer {
    #[must_use]
    pub fn new(attr: LayerAttr, item: LayerItem) -> Self {
        Self(attr, item)
    }

    /// Parses a layer attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`LayerAttr`] or [`LayerItem`].
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Layer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let docs = item.attrs.iter().filter(|attr| attr.path().is_ident("doc"));

        // Marker: the value users register and reference. A unit struct, so
        // `#ident` stays a value usable directly in `router.layer(...)`.
        let marker = quote! {
            #(#docs)*
            #[allow(non_camel_case_types)]
            #vis struct #ident;
        };

        // The user's function, re-emitted as the marker's `handler` associated
        // function. Associated items are reached through the type rather than
        // lexical scope, so `#ident::handler` is callable from the trait
        // implementation below.
        let mut inner = item.clone();
        inner.sig.ident = format_ident!("handler", span = ident.span());
        inner.sig.allow_unused_cx();
        inner.vis = Visibility::Inherited;
        inner
            .attrs
            .push(parse_quote! { #[allow(clippy::unused_async)] });
        let handler = quote! {
            impl #ident {
                #inner
            }
        };

        // The trait implementation dispatching requests to the handler: it
        // calls the handler with the rest of the chain and converts the
        // returned value into a response. A layer with an explicit path is a
        // `Layer`; one without derives its path from the module tree through
        // the module router as a `ModuleLayer`.
        let handle = quote! {
            fn handle<'a>(
                &'a self,
                cx: &'a #topcoat_context::Cx,
                body: #topcoat_router::Body,
                next: #topcoat_router::Next<'a>,
            ) -> #topcoat_router::LayerFuture<'a> {
                ::std::boxed::Box::pin(async move {
                    #topcoat_router::response::AsyncIntoResponse::async_into_response(
                        #ident::handler(cx, body, next).await?,
                        cx,
                    )
                    .await
                })
            }
        };
        let context = RouteContext::context_method(&attr.context);
        let (layer, submit_as) =
            if let Some(path) = attr.path.as_ref().and_then(HandlerPath::absolute) {
                let layer = quote! {
                    impl #topcoat_router::Layer for #ident {
                        fn path(&self) -> ::core::option::Option<&#topcoat_router::Path> {
                            const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                            ::core::option::Option::Some(PATH)
                        }

                        #context

                        #handle
                    }
                };
                (layer, quote! { #topcoat_router::Layer })
            } else {
                let relative_path = attr
                    .path
                    .as_ref()
                    .and_then(HandlerPath::relative_path_method);
                let layer = quote! {
                    impl #topcoat_router::ModuleLayer for #ident {
                        fn module_path(&self) -> &'static str {
                            ::core::module_path!()
                        }

                        #relative_path

                        #context

                        #handle
                    }
                };
                (layer, quote! { #topcoat_router::ModuleLayer })
            };

        // Discovery collects the marker erased behind its trait.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #submit_as } }
        });

        quote! {
            #marker

            const _: () = {
                #handler

                #layer

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_accepts_context_entries_with_or_without_a_path() {
        let attr: LayerAttr = syn::parse_str("with(a)").unwrap();
        assert!(attr.path.is_none());
        assert_eq!(attr.context.len(), 1);
        let attr: LayerAttr = syn::parse_str("\"/admin\", with(a), with(b)").unwrap();
        assert!(attr.path.is_some());
        assert_eq!(attr.context.len(), 2);
    }
}
