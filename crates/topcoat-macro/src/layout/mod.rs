use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat,
    parse::{Parse, ParseStream},
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
    args: Vec<Ident>,
    has_cx: bool,
}

impl Parse for LayoutItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let mut args = Vec::new();
        let mut has_cx = false;
        let mut has_slot = false;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "layout functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) if pi.ident == "slot" => {
                        has_slot = true;
                        args.push(pi.ident.clone());
                    }
                    Pat::Ident(pi) if pi.ident == "cx" => {
                        has_cx = true;
                        args.push(pi.ident.clone());
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "layout functions only accept a `slot: Slot` and an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        if !has_slot {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: Slot` parameter",
            ));
        }
        Ok(Self { item, args, has_cx })
    }
}

pub struct Layout(LayoutAttr, LayoutItem);

impl Layout {
    pub fn new(attr: LayoutAttr, item: LayoutItem) -> Self {
        Self(attr, item)
    }
}

impl ToTokens for Layout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let ident = &item.sig.ident;
        let args = &self.1.args;

        let render = if self.1.has_cx {
            quote! {
                |slot| Box::pin(async move {
                    #item
                    ::topcoat::context::with_context(async |cx| #ident(#(#args),*).await).await
                })
            }
        } else {
            quote! {
                |slot| {
                    #item
                    Box::pin(#ident(#(#args),*))
                }
            }
        };

        match attr.path.as_ref() {
            Some(path) => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::Layout = ::topcoat::router::Layout::new(#path, #render);
            },
            None => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::ModuleLayout = ::topcoat::router::ModuleLayout::new(module_path!(), #render);
            }
        }.to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
