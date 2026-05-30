use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ExprPath;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_path(
        path: &ExprPath,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let ident = path.path.get_ident().ok_or_else(|| {
            syn::Error::new_spanned(path, "only single-identifier paths are supported")
        })?;

        js.push_str(&names.resolve(ident));
        path.to_tokens(rust);
        Ok(())
    }
}
