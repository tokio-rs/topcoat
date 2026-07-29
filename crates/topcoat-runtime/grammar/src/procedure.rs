use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType, Visibility,
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
        let ident = &self.1.item.sig.ident;

        let vis = &self.1.item.vis;
        let docs = self
            .1
            .item
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"));
        let mut item = self.1.item.clone();
        item.vis = Visibility::Inherited;

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

        let id_ident = format_ident!("__TOPCOAT_PROCEDURE_ID_{}", ident);

        quote! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const #id_ident: &'static ::core::primitive::str = {
                const HASH: u64 = #topcoat_runtime::endpoint_id_hash(
                    ::core::env!("CARGO_CRATE_NAME"),
                    ::core::module_path!(),
                    ::core::stringify!(#ident),
                );
                const BYTES: [u8; #topcoat_runtime::ENDPOINT_ID_LEN] =
                    #topcoat_runtime::endpoint_id_hex(HASH);
                match ::core::str::from_utf8(&BYTES) {
                    ::core::result::Result::Ok(id) => id,
                    ::core::result::Result::Err(_) => ::core::panic!("hex is ascii"),
                }
            };

            #(#docs)*
            #[allow(non_upper_case_globals)]
            #vis const #ident: &#topcoat_runtime::Procedure::<(#(#arg_tys,)*), #return_ty> = &#topcoat_runtime::Procedure::new(
                #topcoat_runtime::ProcedureId::new(#id_ident),
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

    fn expand(source: &str) -> String {
        Procedure::parse(TokenStream::new(), source.parse().unwrap())
            .unwrap()
            .to_token_stream()
            .to_string()
    }

    const SOURCE: &str = "async fn double(value: f64) -> Result<f64> { Ok(value * 2.0) }";

    #[test]
    fn expands_to_the_same_id_every_time() {
        assert_eq!(expand(SOURCE), expand(SOURCE));
    }

    #[test]
    fn derives_the_id_from_the_crate_module_and_name() {
        let expanded = expand(SOURCE);
        assert!(expanded.contains("endpoint_id_hash"));
        assert!(expanded.contains("CARGO_CRATE_NAME"));
        assert!(expanded.contains("module_path ! ()"));
        assert!(expanded.contains("stringify ! (double)"));
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
