use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::paths::topcoat_router_macro;

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

        quote! { #topcoat_router_macro::segment!(kind = #kind, rename = #name); }
    }
}

impl Parse for ModuleParam {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            param: input.parse()?,
        })
    }
}

impl ToTokens for ModuleParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let param = &self.param;
        let segment = self.segment();

        quote! {
            #param
            #segment
        }
        .to_tokens(tokens);
    }
}
