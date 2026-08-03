use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::paths::{
    topcoat_error, topcoat_router, topcoat_router_macro, topcoat_view,
};

/// The `not_found!` macro input: an optional URL path prefix.
pub struct NotFound {
    /// The prefix the catch-all page covers; derived from the enclosing module
    /// when absent.
    pub path: Option<LitStr>,
}

impl NotFound {
    /// The served path: the prefix with a `{*rest}` catch-all segment
    /// appended. `None` when the path is module-derived.
    #[must_use]
    pub fn catch_all_path(&self) -> Option<LitStr> {
        self.path.as_ref().map(|path| {
            let prefix = path.value();
            let full = format!("{}/{{*rest}}", prefix.trim_end_matches('/'));
            LitStr::new(&full, path.span())
        })
    }
}

impl Parse for NotFound {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: (!input.is_empty()).then(|| input.parse()).transpose()?,
        })
    }
}

impl ToTokens for NotFound {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let result = quote! { #topcoat_error::Result<#topcoat_view::View> };
        let page = |attr: TokenStream| {
            quote! {
                /// The catch-all page declared by `not_found!`, resolving
                /// every request it serves to a not-found error.
                #[#topcoat_router_macro::page(#attr)]
                pub async fn not_found() -> #result {
                    ::core::result::Result::Err(#topcoat_router::error::not_found().into())
                }
            }
        };

        if let Some(path) = self.catch_all_path() {
            page(quote! { * #path }).to_tokens(tokens);
        } else {
            let page = page(quote! { * });
            quote! {
                /// Serves every unmatched URL under this module as a
                /// not-found error.
                pub mod not_found {
                    #topcoat_router_macro::segment!(kind = CatchAll, rename = "rest");

                    #page
                }
            }
            .to_tokens(tokens);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catch_all_path(source: &str) -> Option<String> {
        let not_found: NotFound = syn::parse_str(source).unwrap();
        not_found.catch_all_path().map(|path| path.value())
    }

    #[test]
    fn empty_input_derives_the_path_from_the_module() {
        assert_eq!(catch_all_path(""), None);
    }

    #[test]
    fn appends_the_catch_all_to_the_prefix() {
        assert_eq!(
            catch_all_path("\"/admin\"").as_deref(),
            Some("/admin/{*rest}"),
        );
    }

    #[test]
    fn root_prefix_serves_a_bare_catch_all() {
        assert_eq!(catch_all_path("\"/\"").as_deref(), Some("/{*rest}"));
    }

    #[test]
    fn trailing_slashes_do_not_double_the_separator() {
        assert_eq!(
            catch_all_path("\"/admin/\"").as_deref(),
            Some("/admin/{*rest}"),
        );
    }

    #[test]
    fn rejects_a_non_string_input() {
        assert!(syn::parse_str::<NotFound>("42").is_err());
    }

    #[test]
    fn rejects_tokens_after_the_path() {
        assert!(syn::parse_str::<NotFound>("\"/admin\" extra").is_err());
    }
}
