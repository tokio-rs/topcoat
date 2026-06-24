use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType,
    parse::{Parse, ParseStream},
};

pub struct ProcedureAttr {}

impl Parse for ProcedureAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

pub struct ProcedureItem {
    item: ItemFn,
}

impl Parse for ProcedureItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
    }
}

pub struct Procedure(ProcedureAttr, ProcedureItem);

impl Procedure {
    #[must_use]
    pub fn new(attr: ProcedureAttr, item: ProcedureItem) -> Self {
        Self(attr, item)
    }

    /// Parses a procedure from its attribute and item token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a procedure
    /// attribute or function item.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Procedure {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.sig.ident;

        let mut args = Vec::new();
        let mut args_with_cx = Vec::new();
        let mut arg_index = 0;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Typed(PatType { pat, .. }) => match &**pat {
                    Pat::Ident(PatIdent { ident, .. }) if ident == "cx" => {
                        args_with_cx.push(ident.clone());
                    }
                    _ => {
                        args.push(format_ident!("arg{arg_index}"));
                        args_with_cx.push(format_ident!("arg{arg_index}"));
                        arg_index += 1;
                    }
                },
                FnArg::Receiver(_) => unreachable!("procedures cannot have `self` receiver"),
            }
        }

        let arg_tys = item
            .sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(PatType { pat, ty, .. }) => match &**pat {
                    Pat::Ident(PatIdent { ident, .. }) if ident == "cx" => None,
                    _ => Some(ty),
                },
                FnArg::Receiver(_) => None,
            })
            .collect::<Vec<_>>();
        let ReturnType::Type(_, return_ty) = &item.sig.output else {
            unreachable!("procedures must return a value")
        };
        let return_ty = quote! { <#return_ty as ::topcoat::internal::ResultExt>::T };

        let id = uuid::Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: &::topcoat::runtime::Procedure::<(#(#arg_tys,)*), #return_ty> = &::topcoat::runtime::Procedure::new(
                ::topcoat::runtime::ProcedureId::new(#id),
                |cx, body| {
                    #[allow(clippy::unused_async)]
                    #item
                    Box::pin(async {
                        type Surrogate = <(#(#arg_tys,)*) as ::topcoat::runtime::Surrogated>::Surrogate;
                        let ::topcoat::router::Json(args) = <::topcoat::router::Json<Surrogate> as topcoat::router::FromRequest>::from_request(cx, body).await?;
                        let (#(#args,)*) = ::topcoat::runtime::Surrogate::into_real(args);
                        let response = ::topcoat::runtime::Surrogated::into_surrogate(#ident(#(#args_with_cx),*).await?);
                        ::topcoat::router::IntoResponse::into_response(::topcoat::router::Json(response))
                    })
                },
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { ::topcoat::runtime::ErasedProcedure::new(#ident) } }.to_tokens(tokens);
        }
    }
}
