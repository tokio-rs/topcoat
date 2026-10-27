use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_router};

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

impl Methods {
    /// Parses one method name. HTTP methods are case-sensitive and the
    /// standard ones are uppercase, so a name with any other character is
    /// rejected, which leaves lowercase names free for other syntax.
    fn parse_method(input: ParseStream) -> syn::Result<Ident> {
        let ident: Ident = input.parse()?;
        if !Self::is_method(&ident) {
            return Err(syn::Error::new(
                ident.span(),
                "expected an uppercase HTTP method, such as `GET`",
            ));
        }
        Ok(ident)
    }

    fn is_method(ident: &Ident) -> bool {
        ident.to_string().bytes().all(|byte| byte.is_ascii_uppercase())
    }
}

impl Parse for Methods {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            Ok(Self::One(input.call(Self::parse_method)?))
        } else if lookahead.peek(Bracket) {
            let items;
            let bracket = syn::bracketed!(items in input);
            let items = items.parse_terminated(Self::parse_method, Token![,])?;
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
        input
            .cursor()
            .ident()
            .is_some_and(|(ident, _)| Self::is_method(&ident))
            || input.peek(Bracket)
            || input.peek(Token![*])
    }
}

impl ToTokens for Methods {
    /// Emits the `Methods` expression declaring this method set.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::One(method) => quote! {
                #topcoat_router::Methods::Only(&[#topcoat_router::Method::#method])
            },
            Self::Set { items, .. } => {
                let items = items
                    .iter()
                    .map(|method| quote! { #topcoat_router::Method::#method });
                quote! { #topcoat_router::Methods::Only(&[#(#items),*]) }
            }
            Self::Any(_) => quote! { #topcoat_router::Methods::Any },
        }
        .to_tokens(tokens);
    }
}
