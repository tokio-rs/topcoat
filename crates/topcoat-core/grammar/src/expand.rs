//! Helpers for the output of macro entry points.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{FnArg, ItemFn};

/// Expands to `error` followed by the original `item`, for an attribute macro
/// whose input could not be parsed or expanded.
///
/// Keeping the item lets the rest of the crate and IDEs such as
/// rust-analyzer still see it while the error is being fixed. Attributes named
/// in `helper_attributes` are removed from a function's parameters, since they
/// only have meaning to the macro and would otherwise be reported as unknown.
#[must_use]
pub fn item_with_error(
    error: &syn::Error,
    item: TokenStream,
    helper_attributes: &[&str],
) -> TokenStream {
    let mut tokens = error.to_compile_error();
    let parsed = if helper_attributes.is_empty() {
        None
    } else {
        syn::parse2::<ItemFn>(item.clone()).ok()
    };
    if let Some(mut parsed) = parsed {
        for input in &mut parsed.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                pat_type.attrs.retain(|attr| {
                    !helper_attributes
                        .iter()
                        .any(|name| attr.path().is_ident(name))
                });
            }
        }
        parsed.to_tokens(&mut tokens);
    } else {
        tokens.extend(item);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::{File, Item, Pat};

    use super::*;

    fn expand(item: TokenStream, helper_attributes: &[&str]) -> File {
        let error = syn::Error::new(proc_macro2::Span::call_site(), "invalid");
        syn::parse2(item_with_error(&error, item, helper_attributes)).unwrap()
    }

    fn function(file: &File) -> &ItemFn {
        file.items
            .iter()
            .find_map(|item| match item {
                Item::Fn(item) => Some(item),
                _ => None,
            })
            .expect("the item is kept")
    }

    #[test]
    fn keeps_the_item_next_to_the_error() {
        let file = expand(quote! { async fn card() -> u8 { 1 } }, &[]);
        assert!(file.items.iter().any(|item| matches!(
            item,
            Item::Macro(item)
                if item.mac.path.segments.last().unwrap().ident == "compile_error"
        )));
        assert_eq!(function(&file).sig.ident, "card");
    }

    #[test]
    fn keeps_an_item_that_does_not_parse() {
        let item = quote! { async fn card(#[default] title: u8) { user. } };
        let error = syn::Error::new(proc_macro2::Span::call_site(), "invalid");
        let tokens = item_with_error(&error, item.clone(), &["default"]);
        assert!(tokens.to_string().ends_with(&item.to_string()));
    }

    #[test]
    fn removes_only_helper_attributes_from_parameters() {
        let file = expand(
            quote! { async fn card(#[default] #[cfg(test)] title: u8, #[into] body: u8) {} },
            &["default", "into"],
        );
        let names: Vec<_> = function(&file)
            .sig
            .inputs
            .iter()
            .map(|input| {
                let FnArg::Typed(pat_type) = input else {
                    unreachable!()
                };
                let Pat::Ident(pat) = &*pat_type.pat else {
                    unreachable!()
                };
                let attrs: Vec<_> = pat_type
                    .attrs
                    .iter()
                    .map(|attr| attr.path().get_ident().unwrap().to_string())
                    .collect();
                (pat.ident.to_string(), attrs)
            })
            .collect();
        assert_eq!(
            names,
            [
                ("title".to_owned(), vec!["cfg".to_owned()]),
                ("body".to_owned(), vec![]),
            ]
        );
    }
}
