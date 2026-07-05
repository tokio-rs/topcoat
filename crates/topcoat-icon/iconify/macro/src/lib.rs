use proc_macro::TokenStream;
use quote::quote;

/// Expands to `const` icons from a staged Iconify icon set.
///
/// Accepts `"set"` (a module with a const per icon), `"set:*"` (the consts
/// inlined into the current scope), or `"set:icon"` (a single const), with an
/// optional leading visibility: `include!(pub(crate) "mdi")`. Sets are staged
/// by the consuming crate's build script through
/// `topcoat::icon::iconify::Sets`.
#[proc_macro]
pub fn include(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_iconify::ast::Include);
    quote! { #parsed }.into()
}

/// Expands to a single icon of a staged Iconify icon set as a
/// const-evaluable `IconData` expression.
///
/// Accepts a `"set:icon"` reference. Sets are staged by the consuming
/// crate's build script through `topcoat::icon::iconify::Sets`.
#[proc_macro]
pub fn iconify_icon(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_iconify::ast::IconifyIcon);
    quote! { #parsed }.into()
}
