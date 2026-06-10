use std::ops::Deref;

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType,
    parse::{Parse, ParseStream},
};

pub struct ActionAttr {}

impl Parse for ActionAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

pub struct ActionItem {
    item: ItemFn,
}

impl Parse for ActionItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
    }
}

pub struct Action(ActionAttr, ActionItem);

impl Action {
    pub fn new(attr: ActionAttr, item: ActionItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Action {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.sig.ident;

        let mut args = Vec::new();
        let mut args_with_cx = Vec::new();
        let mut arg_index = 0;
        for arg in item.sig.inputs.iter() {
            match arg {
                FnArg::Typed(PatType { pat, .. }) => match pat.deref() {
                    Pat::Ident(PatIdent { ident, .. }) if ident == "cx" => {
                        args_with_cx.push(ident.clone());
                    }
                    _ => {
                        args.push(format_ident!("arg{arg_index}"));
                        args_with_cx.push(format_ident!("arg{arg_index}"));
                        arg_index += 1;
                    }
                },
                _ => unreachable!("actions cannot have `self` receiver"),
            }
        }

        let arg_tys = item
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(PatType { pat, ty, .. }) => match pat.deref() {
                    Pat::Ident(PatIdent { ident, .. }) if ident == "cx" => None,
                    _ => Some(ty),
                },
                _ => None,
            })
            .collect::<Vec<_>>();
        let ReturnType::Type(_, return_ty) = &item.sig.output else {
            unreachable!("actions must return a value")
        };
        let return_ty = quote! { <#return_ty as ::topcoat::internal::ResultExt>::T };

        let id = uuid::Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: &::topcoat::router::Action::<(#(#arg_tys,)*), #return_ty> = &::topcoat::router::Action::new(
                ::topcoat::router::ActionId::new(#id),
                |cx, body| {
                    #item
                    Box::pin(async {
                        type Surrogate = <(#(#arg_tys,)*) as ::topcoat::runtime::Surrogated>::Surrogate;
                        let ::topcoat::router::Json(args) = <::topcoat::router::Json<Surrogate> as topcoat::router::FromRequest>::from_request(cx, body).await?;
                        let (#(#args,)*) = ::topcoat::runtime::Surrogate::into_real(args);
                        ::topcoat::router::IntoResponse::into_response(::topcoat::router::Json(#ident(#(#args_with_cx),*).await?))
                    })
                },
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { ::topcoat::router::ErasedAction::new(#ident) } }.to_tokens(tokens);
        }
    }
}
