use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use topcoat_icon::iconify::ResolvedIcon;

use crate::iconify::{
    Selected, Selection,
    codegen::{icon_expr, resolve_icon},
    staged::staged_set,
};

/// One `iconify_icon!` invocation: a single-icon [`Selection`] like
/// `"mdi:delete"`, expanding to a const-evaluable `IconData` expression.
pub struct IconifyIcon {
    icon: ResolvedIcon<'static>,
}

impl Parse for IconifyIcon {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let selection: Selection = input.parse()?;
        let Selected::Icon(name) = &selection.selected else {
            return Err(syn::Error::new(
                selection.span(),
                "expected a single icon, like `\"mdi:delete\"`",
            ));
        };
        let set = staged_set(&selection.prefix, selection.span())?;
        Ok(Self {
            icon: resolve_icon(set, name, selection.span())?,
        })
    }
}

impl ToTokens for IconifyIcon {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        icon_expr(&self.icon).to_tokens(tokens);
    }
}
