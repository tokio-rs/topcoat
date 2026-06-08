use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprPath;

use crate::ast::expr::{
    Expr,
    name_resolver::{NameResolver, ResolvedIdent},
};

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

        if ident == "None" {
            js.push_str("cx.none()");
            quote! { ::topcoat::runtime::Option::<_>::none() }.to_tokens(rust);
            return Ok(());
        }

        let resolved = names.resolve(ident);
        let (js_name, rust_ident) = match resolved {
            ResolvedIdent::Local {
                js_name,
                rust_ident,
            }
            | ResolvedIdent::External {
                js_name,
                rust_ident,
            } => (js_name, rust_ident),
        };

        js.push_str(&js_name);
        rust_ident.to_tokens(rust);
        Ok(())
    }
}
