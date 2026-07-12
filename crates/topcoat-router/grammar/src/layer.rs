use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    ItemFn, LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::paths::{topcoat_inventory, topcoat_router};

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
        let ident = &item.sig.ident;

        let render = quote! {
            |cx, body, next| {
                #[allow(clippy::unused_async)]
                #item
                Box::pin(async move {
                    #topcoat_router::IntoResponse::into_response(#ident(cx, body, next).await?, cx)
                })
            }
        };

        if let Some(path) = attr.path.as_ref() {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::LayerFn = #topcoat_router::LayerFn::new(
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );
            }
        } else {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::ModuleLayerFn = #topcoat_router::ModuleLayerFn::new(module_path!(), #render);
            }
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { #topcoat_inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
