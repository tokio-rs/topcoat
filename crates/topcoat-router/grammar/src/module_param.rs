use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::common::paths::{topcoat_router, topcoat_router_macro};

use super::path_param::PathParam;

/// A path parameter declaration that also sets the module's URL segment.
pub struct ModuleParam {
    pub param: PathParam,
}

impl ModuleParam {
    fn segment(&self) -> TokenStream {
        let name = self.param.name_string();
        let kind = if self.param.is_catch_all() {
            quote! { CatchAll }
        } else {
            quote! { Param }
        };

        quote! { #topcoat_router_macro::segment!(kind: #kind, rename: #name); }
    }

    /// The module-level `__href` function, forwarding to the marker's.
    ///
    /// A module named after its parameter shadows the marker in the parent
    /// module, so `href!(post_id::page, post_id: 5)` written there expands to
    /// `post_id::__href(5)` and reaches this function through the module.
    fn href_fn(&self) -> TokenStream {
        let visibility = &self.param.visibility;
        let ident = &self.param.name;
        let bound = self.param.value_bound();

        quote! {
            #[doc(hidden)]
            #[allow(dead_code)]
            #visibility fn __href<V: #bound>(value: V) -> #topcoat_router::HrefArg<#ident, V> {
                #ident::__href(value)
            }
        }
    }
}

impl Parse for ModuleParam {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            param: input.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for ModuleParam {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.param.pretty_print(printer);
    }
}

impl ToTokens for ModuleParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let param = &self.param;
        let segment = self.segment();
        let href_fn = self.href_fn();

        quote! {
            #param
            #segment
            #href_fn
        }
        .to_tokens(tokens);
    }
}
