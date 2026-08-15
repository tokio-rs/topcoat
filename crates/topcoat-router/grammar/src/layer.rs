use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    ItemFn, LitStr, Visibility,
    parse::{Parse, ParseStream},
    parse_quote,
};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_inventory, topcoat_router};

pub struct LayerAttr {
    path: Option<LitStr>,
}

impl Parse for LayerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct LayerItem {
    item: ItemFn,
}

impl Parse for LayerItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
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
                    #topcoat_router::response::IntoResponse::into_response(
                        #ident::handler(cx, body, next).await?,
                        cx,
                    )
                })
            }
        };
        let (layer, submit_as) = if let Some(path) = attr.path.as_ref() {
            let layer = quote! {
                impl #topcoat_router::Layer for #ident {
                    fn path(&self) -> ::core::option::Option<&#topcoat_router::Path> {
                        const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                        ::core::option::Option::Some(PATH)
                    }

                    #handle
                }
            };
            (layer, quote! { #topcoat_router::Layer })
        } else {
            let layer = quote! {
                impl #topcoat_router::ModuleLayer for #ident {
                    fn module_path(&self) -> &'static str {
                        ::core::module_path!()
                    }

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
