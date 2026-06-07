mod block;
mod builtin_macro;
mod expr_binary;
mod expr_block;
mod expr_closure;
mod expr_field;
mod expr_index;
mod expr_lit;
mod expr_method_call;
mod expr_paren;
mod expr_path;
mod expr_unary;
mod name_resolver;
mod pat;
mod stmt;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};

use crate::ast::expr::name_resolver::NameResolver;

/// The top-level `expr! { ... }` AST. A thin wrapper around `syn::Expr`; the
/// whitelist of supported shapes is enforced when lowering to tokens.
pub struct Expr {
    inner: syn::Expr,
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            inner: input.parse()?,
        })
    }
}

impl Expr {
    pub fn expr_to_tokens(&self) -> syn::Result<TokenStream> {
        let mut rust = TokenStream::new();
        let mut js = String::new();
        let mut names = NameResolver::default();
        Self::dispatch(&self.inner, &mut rust, &mut js, &mut names)?;

        if !matches!(self.inner, syn::Expr::Closure(..)) {
            rust = quote! { ::topcoat::runtime::Surrogate::into_real(#rust) }
        }

        // Identifiers referenced but not declared by the expression are
        // captured from the surrounding Rust scope. Their values are encoded
        // into the JavaScript source at runtime as `const` bindings, declared
        // ahead of the returned expression.
        let externals = names.externals();

        if !externals.is_empty() {
            let rust_external_idents = externals.iter().map(|binding| &binding.rust_ident);
            let rust_external_values = externals.iter().map(|binding| {
                let ident = &binding.original_ident;
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#ident) }
            });

            let mut js_head = "(() => { const [".to_owned();
            for (index, binding) in externals.iter().enumerate() {
                js_head += &binding.js_name;
                if index < externals.len() - 1 {
                    js_head += ", ";
                }
            }
            js_head += "] = [";

            let mut js_externals = TokenStream::new();
            for (index, binding) in externals.iter().enumerate() {
                let rust_ident = &binding.rust_ident;
                quote! { __surrogate(&mut __parts, &#rust_ident); }.to_tokens(&mut js_externals);
                if index < externals.len() - 1 {
                    quote! { __js_unescaped(&mut __parts, ", "); }.to_tokens(&mut js_externals);
                }
            }

            let js_tail = "]; return ".to_owned() + &js + "; })()";

            Ok(quote! {{
                use ::topcoat::runtime::internal::*;

                let (#(#rust_external_idents,)*) = (#(#rust_external_values,)*);
                let mut __parts = ::topcoat::view::ViewParts::new();
                __js_unescaped(&mut __parts, #js_head);
                #js_externals
                let __rust = #rust;
                __js(&mut __parts, #js_tail);
                ::topcoat::runtime::Expr::new(__rust, ::topcoat::view::ViewPart::from(__parts))
            }})
        } else {
            Ok(quote! {
                ::topcoat::runtime::Expr::new(#rust, ::topcoat::view::ViewPart::from(#js))
            })
        }
    }

    /// Lowers a single `syn::Expr` into a Rust value (`rust`) and the
    /// equivalent JavaScript source (`js`), recursing into sub-expressions.
    fn dispatch(
        expr: &syn::Expr,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match expr {
            syn::Expr::Lit(inner) => Self::expr_lit(inner, rust, js)?,
            syn::Expr::Paren(inner) => Self::expr_paren(inner, rust, js, names)?,
            syn::Expr::Binary(inner) => Self::expr_binary(inner, rust, js, names)?,
            syn::Expr::Unary(inner) => Self::expr_unary(inner, rust, js, names)?,
            syn::Expr::MethodCall(inner) => Self::expr_method_call(inner, rust, js, names)?,
            syn::Expr::Field(inner) => Self::expr_field(inner, rust, js, names)?,
            syn::Expr::Index(inner) => Self::expr_index(inner, rust, js, names)?,
            syn::Expr::Block(inner) => Self::expr_block(inner, rust, js, names)?,
            syn::Expr::Closure(inner) => Self::expr_closure(inner, rust, js, names)?,
            syn::Expr::Path(inner) => Self::expr_path(inner, rust, js, names)?,
            syn::Expr::Macro(inner) => Self::expr_macro(inner, rust, js, names)?,
            other => return Err(syn::Error::new_spanned(other, "unsupported expression")),
        }
        Ok(())
    }
}
