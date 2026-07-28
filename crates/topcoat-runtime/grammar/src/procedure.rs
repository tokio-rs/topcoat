use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{
    topcoat_internal, topcoat_inventory, topcoat_router, topcoat_runtime,
};

pub struct ProcedureAttr {}

impl Parse for ProcedureAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

/// The annotated `async fn` that becomes a procedure. Validates the function
/// signature: procedures must be `async`, must declare a return type, and must
/// not take a `self` receiver.
pub struct ProcedureItem {
    item: ItemFn,
}

impl Parse for ProcedureItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "procedures must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "procedures must have a return type",
            ));
        }
        for arg in &item.sig.inputs {
            if let FnArg::Receiver(r) = arg {
                return Err(syn::Error::new_spanned(
                    r,
                    "procedure functions cannot take a `self` receiver",
                ));
            }
        }
        Ok(Self { item })
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
                FnArg::Receiver(_) => unreachable!("validated by ProcedureItem"),
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
            unreachable!("validated by ProcedureItem")
        };
        let return_ty = quote! { <#return_ty as #topcoat_internal::ResultExt>::T };

        let id = uuid::Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: &#topcoat_runtime::Procedure::<(#(#arg_tys,)*), #return_ty> = &#topcoat_runtime::Procedure::new(
                #topcoat_runtime::ProcedureId::new(#id),
                |cx, body| {
                    #[allow(clippy::unused_async)]
                    #item
                    Box::pin(async {
                        type Surrogate = <(#(#arg_tys,)*) as #topcoat_runtime::Surrogated>::Surrogate;
                        let #topcoat_router::content::Json(args) = <#topcoat_router::content::Json<Surrogate> as #topcoat_router::FromRequest>::from_request(cx, body).await?;
                        let (#(#args,)*) = #topcoat_runtime::Surrogate::into_real(args);
                        let response = #topcoat_runtime::Surrogated::into_surrogate(#ident(#(#args_with_cx),*).await?);
                        #topcoat_router::IntoResponse::into_response(#topcoat_router::content::Json(response), cx)
                    })
                },
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { #topcoat_inventory::submit! { #topcoat_runtime::ErasedProcedure::new(#ident) } }.to_tokens(tokens);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<ProcedureItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<ProcedureItem>("async fn double(cx: &Cx, value: f64) -> Result<f64> {}")
            .unwrap();
    }

    #[test]
    fn accepts_a_destructured_argument() {
        // Only argument types reach the generated call, so destructuring
        // patterns stay valid; the re-emitted function unpacks them.
        syn::parse_str::<ProcedureItem>(
            "async fn shift((x, y): (f64, f64)) -> Result<(f64, f64)> {}",
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn double() -> Result<f64> {}").contains("must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn double() {}").contains("must have a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn double(&self) -> Result<f64> {}");
        assert!(err.contains("cannot take a `self` receiver"));
    }
}
