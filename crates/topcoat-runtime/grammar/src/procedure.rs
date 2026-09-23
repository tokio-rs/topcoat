use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, ItemFn, Pat, PatIdent, PatType, ReturnType, Visibility,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{
        topcoat_context, topcoat_error, topcoat_internal, topcoat_inventory, topcoat_router,
        topcoat_runtime,
    },
};

use crate::common::EndpointPath;

/// The prefix below which a procedure without a path of its own is served.
const PROCEDURE_ROUTE_PREFIX: &str = "/_topcoat/runtime/procedures";

/// Arguments to `#[procedure]`: an optional path the procedure is served
/// at, as in `#[procedure("/api/double")]`.
pub struct ProcedureAttr {
    pub path: Option<EndpointPath>,
}

impl Parse for ProcedureAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.call(EndpointPath::parse_option)?,
        })
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
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let docs = item.attrs.iter().filter(|attr| attr.path().is_ident("doc"));

        // Marker: the value users register and reference. A unit struct, so
        // `#ident` stays a value usable directly in `router.procedure(...)`
        // and capturable in runtime expressions. `Copy` lets the surrogate
        // hand the marker back out of its `&'static` reference.
        let marker = quote! {
            #(#docs)*
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            #vis struct #ident;
        };

        // The user's function, re-emitted under its original name inside the
        // bridge below to keep the module namespace clean. Its own name
        // shadows the marker within its body, so bindings named after the
        // procedure keep working.
        let mut inner = item.clone();
        inner.vis = Visibility::Inherited;
        inner
            .attrs
            .push(parse_quote! { #[allow(clippy::unused_async)] });

        // The bridge every caller goes through: it deserializes the surrogate
        // argument tuple from the request body, forwards to the user's
        // function positionally, and serializes the returned value back into
        // a surrogate response.
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
        let handler = quote! {
            impl #ident {
                async fn handler(
                    cx: &#topcoat_context::Cx,
                    body: #topcoat_router::Body,
                ) -> #topcoat_error::Result<#topcoat_router::response::Response> {
                    #inner

                    type Surrogate = <(#(#arg_tys,)*) as #topcoat_runtime::Surrogated>::Surrogate;
                    let #topcoat_router::content::Json(#topcoat_runtime::Arguments(args)) =
                        <#topcoat_router::content::Json<#topcoat_runtime::Arguments<Surrogate>> as #topcoat_router::request::FromRequest>::from_request(cx, body).await?;
                    let (#(#args,)*) = #topcoat_runtime::Surrogate::into_real(args);
                    let response = #topcoat_runtime::Surrogated::into_surrogate(#ident(#(#args_with_cx),*).await?);
                    #topcoat_router::response::IntoResponse::into_response(#topcoat_router::content::Json(response), cx)
                }
            }
        };

        // The route serving calls at the endpoint's path, dispatching them to
        // the bridge.
        let path = EndpointPath::resolve(self.0.path.as_ref(), PROCEDURE_ROUTE_PREFIX);
        let route = quote! {
            impl #topcoat_router::Route for #ident {
                fn id(&self) -> #topcoat_router::RouteId {
                    *ID
                }

                fn methods(&self) -> #topcoat_router::Methods<'_> {
                    const METHODS: #topcoat_router::Methods<'static> =
                        #topcoat_router::Methods::Only(&[#topcoat_router::Method::POST]);
                    METHODS
                }

                fn path(&self) -> &#topcoat_router::Path {
                    const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                    PATH
                }

                fn handle<'cx>(
                    &'cx self,
                    cx: &'cx #topcoat_context::Cx,
                    body: #topcoat_router::Body,
                ) -> #topcoat_router::RouteFuture<'cx> {
                    ::std::boxed::Box::pin(#ident::handler(cx, body))
                }
            }
        };

        // Runtime expressions capture the marker as a typed surrogate, so
        // calls in `expr!` bodies check their arguments against the declared
        // parameter types.
        let ReturnType::Type(_, return_ty) = &item.sig.output else {
            unreachable!("validated by ProcedureItem")
        };
        let return_ty = quote! { <#return_ty as #topcoat_internal::ResultExt>::T };
        let typed = quote! {
            impl #topcoat_runtime::TypedProcedure for #ident {
                type Args = (#(#arg_tys,)*);
                type Output = #return_ty;
            }

            impl #topcoat_runtime::Surrogated for #ident {
                type Surrogate = &'static #topcoat_runtime::ProcedureSurrogate<#ident>;

                fn into_surrogate(self) -> Self::Surrogate {
                    static SURROGATE: #topcoat_runtime::ProcedureSurrogate<#ident> =
                        #topcoat_runtime::ProcedureSurrogate::new(#ident);
                    &SURROGATE
                }
            }
        };

        // Discovery collects the marker as a route.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #topcoat_router::Route } }
        });

        quote! {
            #marker

            const _: () = {
                static ID: ::std::sync::LazyLock<#topcoat_router::RouteId> =
                    ::std::sync::LazyLock::new(#topcoat_router::RouteId::new);

                #handler

                #route

                #typed

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

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

    #[test]
    fn a_named_path_replaces_the_default_route() {
        let procedure = Procedure::parse(
            quote! { "/api/double" },
            quote! { async fn double(value: f64) -> Result<f64> { todo!() } },
        )
        .unwrap();
        let out = procedure.to_token_stream().to_string();
        assert!(out.contains(r#""/api/double""#), "{out}");
        assert!(!out.contains(PROCEDURE_ROUTE_PREFIX), "{out}");
    }

    #[test]
    fn a_procedure_without_a_path_is_served_below_the_default_prefix() {
        let procedure = Procedure::parse(
            TokenStream::new(),
            quote! { async fn double(value: f64) -> Result<f64> { todo!() } },
        )
        .unwrap();
        let out = procedure.to_token_stream().to_string();
        assert!(out.contains(PROCEDURE_ROUTE_PREFIX), "{out}");
    }
}
