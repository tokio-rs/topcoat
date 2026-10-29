use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};

use crate::iconify::{
    Selected, Selection,
    codegen::{icon_expr, resolve_icon},
    staged::staged_set,
};

/// One `iconify_icon!` invocation: a single-icon [`Selection`] like
/// `"mdi:delete"`, expanding to a const-evaluable `IconData` expression.
pub struct IconifyIcon {
    pub selection: Selection,
}

impl IconifyIcon {
    /// The name of the selected icon. Fails when the selection names a whole
    /// set instead of a single icon.
    fn icon_name(selection: &Selection) -> syn::Result<&str> {
        match &selection.selected {
            Selected::Icon(name) => Ok(name),
            Selected::Set | Selected::Glob => Err(syn::Error::new(
                selection.span(),
                "expected a single icon, like `\"mdi:delete\"`",
            )),
        }
    }

    /// Expands to the icon's data. Fails when the icon set is not staged or
    /// does not contain the icon.
    fn expand(&self) -> syn::Result<TokenStream> {
        let name = Self::icon_name(&self.selection)?;
        let set = staged_set(&self.selection.prefix, self.selection.span())?;
        Ok(icon_expr(&resolve_icon(set, name, self.selection.span())?))
    }
}

impl Parse for IconifyIcon {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let selection: Selection = input.parse()?;
        Self::icon_name(&selection)?;
        Ok(Self { selection })
    }
}

impl ToTokens for IconifyIcon {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expand()
            .unwrap_or_else(|error| error.to_compile_error())
            .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for IconifyIcon {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.selection.pretty_print(printer);
    }
}
