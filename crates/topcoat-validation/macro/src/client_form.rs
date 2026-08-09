use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

pub fn derive(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;
    let validator_ident = format_ident!(
        "__topcoat_validate_{}",
        ident.to_string().to_snake_case()
    );

    quote! {
        #[::topcoat::runtime::procedure]
        async fn #validator_ident(form_data: ::std::string::String) -> ::topcoat::Result<::std::string::String> {
            ::topcoat::validation::validate_client::<#ident>(form_data).await
        }
    }
}
