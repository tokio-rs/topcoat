use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_cookie};

use crate::cookie::{CookieValue, HeaderName};

/// An attribute after the cookie's value, such as `Path = "/"` or the flag
/// `Secure`.
///
/// The value is optional during parsing, so an attribute that is still being typed
/// keeps the call parsing.
pub struct CookieAttribute {
    pub name: HeaderName,
    pub eq_token: Option<Token![=]>,
    pub value: Option<CookieValue>,
}

impl CookieAttribute {
    /// A call of the `Cookie` setter for this attribute on `cookie`, or a compile
    /// error if the attribute or its value is not supported.
    ///
    /// The setter keeps the span of the attribute name, so editors show the
    /// setter's documentation for it.
    pub(crate) fn apply(&self, cookie: &Ident) -> TokenStream {
        let name = self.name.value();
        let setter = match name.as_str() {
            "Path" => "set_path",
            "Domain" => "set_domain",
            "Max-Age" => "set_max_age",
            "Expires" => "set_expires",
            "SameSite" => "set_same_site",
            "Secure" => "set_secure",
            "HttpOnly" => "set_http_only",
            "Partitioned" => "set_partitioned",
            _ => {
                return syn::Error::new(
                    self.name.span(),
                    format!("unknown cookie attribute `{name}`"),
                )
                .to_compile_error();
            }
        };
        let setter = Ident::new(setter, self.name.span());
        match self.argument(&name) {
            Ok(argument) => quote! { #cookie.#setter(#argument); },
            Err(error) => error.to_compile_error(),
        }
    }

    /// The argument of the setter for the attribute called `name`.
    fn argument(&self, name: &str) -> syn::Result<TokenStream> {
        let is_flag = matches!(name, "Secure" | "HttpOnly" | "Partitioned");
        match &self.value {
            // A flag on its own turns the attribute on.
            None if is_flag && self.eq_token.is_none() => Ok(quote! { true }),
            None => {
                let span = self.eq_token.map_or(self.name.span(), |eq| eq.span);
                Err(syn::Error::new(
                    span,
                    format!("expected a value for `{name}`"),
                ))
            }
            Some(CookieValue::Name(variant)) if name == "SameSite" => {
                Ok(quote! { #topcoat_cookie::SameSite::#variant })
            }
            // An integer counts seconds, as in the header.
            Some(CookieValue::Lit(seconds)) if name == "Max-Age" => {
                Ok(quote! { #topcoat_cookie::time::Duration::seconds(#seconds) })
            }
            Some(value) => value.expr(),
        }
    }
}

impl Parse for CookieAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let eq_token: Option<Token![=]> = input.parse()?;
        let value = match eq_token {
            Some(_) => input.call(CookieValue::parse_option)?,
            None => None,
        };
        Ok(Self {
            name,
            eq_token,
            value,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for CookieAttribute {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.name.pretty_print(printer);
        if let Some(eq_token) = &self.eq_token {
            " ".pretty_print(printer);
            eq_token.pretty_print(printer);
        }
        if let Some(value) = &self.value {
            " ".pretty_print(printer);
            value.pretty_print(printer);
        }
    }
}
