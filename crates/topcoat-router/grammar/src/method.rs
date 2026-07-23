use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};
use topcoat_core_grammar::ParseOption;
use topcoat_core_grammar::paths::topcoat_router;

/// The HTTP methods opening a route-like macro attribute: a single method
/// (`GET`), a bracketed list (`[GET, POST]`), or `*` for every method.
pub enum Methods {
    /// A single method (`#[route(GET)]`).
    One(Ident),
    /// A bracketed list of methods (`#[route([GET, POST])]`).
    Set {
        bracket: Bracket,
        items: Punctuated<Ident, Token![,]>,
    },
    /// Every method (`#[route(*)]`).
    Any(Token![*]),
}

impl Parse for Methods {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            Ok(Self::One(input.parse()?))
        } else if lookahead.peek(Bracket) {
            let items;
            let bracket = syn::bracketed!(items in input);
            let items = items.parse_terminated(Ident::parse, Token![,])?;
            if items.is_empty() {
                return Err(syn::Error::new(
                    bracket.span.join(),
                    "expected at least one HTTP method",
                ));
            }
            Ok(Self::Set { bracket, items })
        } else if lookahead.peek(Token![*]) {
            Ok(Self::Any(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ParseOption for Methods {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident) || input.peek(Bracket) || input.peek(Token![*])
    }
}

impl ToTokens for Methods {
    /// Emits the `OwnedMethods` expression registering this method set.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::One(method) => quote! {
                #topcoat_router::OwnedMethods::One(#topcoat_router::Method::#method)
            },
            Self::Set { items, .. } => {
                let items = items
                    .iter()
                    .map(|method| quote! { #topcoat_router::Method::#method });
                quote! {
                    #topcoat_router::OwnedMethods::Set(::std::borrow::Cow::Borrowed(
                        &[#(#items),*],
                    ))
                }
            }
            Self::Any(_) => quote! { #topcoat_router::OwnedMethods::Any },
        }
        .to_tokens(tokens);
    }
}
