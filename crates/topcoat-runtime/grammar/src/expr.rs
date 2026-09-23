mod block;
mod builtin_macro;
mod contains_await;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_continue;
mod expr_field;
mod expr_if;
mod expr_index;
mod expr_lit;
mod expr_loop;
mod expr_method_call;
mod expr_paren;
mod expr_path;
mod expr_return;
mod expr_unary;
mod expr_while;
mod js;
mod name_resolver;
mod pat;
mod stmt;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::paths::topcoat_runtime;

use crate::expr::{js::Js, name_resolver::NameResolver};

/// A parsed `expr! { ... }` body. Lowering checks which expression forms
/// are supported.
pub struct Expr {
    pub inner: syn::Expr,
    pub comma_token: Option<syn::Token![,]>,
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            inner: input.parse()?,
            comma_token: input.parse()?,
        })
    }
}

impl Expr {
    /// Lowers the parsed expression into a token stream.
    ///
    /// # Errors
    ///
    /// Returns an error if any sub-expression is not a supported shape, or if a
    /// local binding cannot be resolved.
    pub fn expr_to_tokens(&self) -> syn::Result<TokenStream> {
        let mut rust = TokenStream::new();
        let mut js = Js::default();
        let mut names = NameResolver::default();
        Self::dispatch(&self.inner, &mut rust, &mut js, &mut names)?;

        if !matches!(self.inner, syn::Expr::Closure(..)) {
            rust = quote! { #topcoat_runtime::Surrogate::into_real(#rust) }
        }

        // Clone each capture once. Its JavaScript is inlined at each use;
        // its cached server value becomes an ordinary surrogate local.
        let externals = names.externals();
        let captures = externals.iter().map(|binding| {
            let ident = &binding.rust_ident;
            let value = &binding.value;
            quote! { let #ident = #value; }
        });
        let values = externals.iter().map(|binding| {
            let ident = &binding.rust_ident;
            quote! { let #ident = #ident.into_captured_value(); }
        });

        Ok(quote! {{
            #(#captures)*
            let __js = #js;
            #topcoat_runtime::Expr::evaluate(|| {
                #(#values)*
                #rust
            }, __js)
        }})
    }

    /// Lowers a single `syn::Expr` into a Rust value (`rust`) and the
    /// equivalent JavaScript source (`js`), recursing into sub-expressions.
    fn dispatch(
        expr: &syn::Expr,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match expr {
            syn::Expr::Await(inner) => Self::expr_await(inner, rust, js, names)?,
            syn::Expr::Lit(inner) => Self::expr_lit(inner, rust, js, names)?,
            syn::Expr::Paren(inner) => Self::expr_paren(inner, rust, js, names)?,
            syn::Expr::Binary(inner) => Self::expr_binary(inner, rust, js, names)?,
            syn::Expr::Unary(inner) => Self::expr_unary(inner, rust, js, names)?,
            syn::Expr::MethodCall(inner) => Self::expr_method_call(inner, rust, js, names)?,
            syn::Expr::Call(inner) => Self::expr_call(inner, rust, js, names)?,
            syn::Expr::Field(inner) => Self::expr_field(inner, rust, js, names)?,
            syn::Expr::Index(inner) => Self::expr_index(inner, rust, js, names)?,
            syn::Expr::Block(inner) => Self::expr_block(inner, rust, js, names)?,
            syn::Expr::Closure(inner) => Self::expr_closure(inner, rust, js, names)?,
            syn::Expr::If(inner) => Self::expr_if(inner, rust, js, names)?,
            syn::Expr::Loop(inner) => Self::expr_loop(inner, rust, js, names)?,
            syn::Expr::While(inner) => Self::expr_while(inner, rust, js, names)?,
            syn::Expr::Continue(inner) => Self::expr_continue(inner, rust, js, names)?,
            syn::Expr::Break(inner) => Self::expr_break(inner, rust, js, names)?,
            syn::Expr::Return(inner) => Self::expr_return(inner, rust, js, names)?,
            syn::Expr::Path(inner) => Self::expr_path(inner, rust, js, names)?,
            syn::Expr::Macro(inner) => Self::expr_macro(inner, rust, js, names)?,
            other => return Err(syn::Error::new_spanned(other, "unsupported expression")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_trailing_comma_preserves_the_expression() {
        for source in [
            "true",
            "if dark.get() { \"Light\" } else { \"Dark\" }",
            "|_e| open.toggle()",
            "async |_e| submit().await",
        ] {
            let plain: Expr = syn::parse_str(source).unwrap();
            let trailing: Expr = syn::parse_str(&format!("{source},")).unwrap();

            assert!(plain.comma_token.is_none());
            assert!(trailing.comma_token.is_some());
            assert_eq!(
                plain.expr_to_tokens().unwrap().to_string(),
                trailing.expr_to_tokens().unwrap().to_string(),
            );
        }
    }

    #[test]
    fn rejects_missing_or_multiple_expressions() {
        for source in ["", ",", "true, false", "true,,", "true;"] {
            assert!(syn::parse_str::<Expr>(source).is_err(), "{source}");
        }
    }
}
