use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::common::lenient;

/// A single `name: value` option in an `asset!` call.
///
/// The colon and value are optional during parsing, so a name that is still being
/// typed keeps the call parsing.
pub struct AssetOption {
    pub name: Ident,
    pub colon_token: Option<Token![:]>,
    pub value: lenient::Expr,
}

impl AssetOption {
    /// The option name as written, without a raw-identifier prefix.
    #[must_use]
    pub fn name(&self) -> String {
        self.name.unraw().to_string()
    }

    /// The option as a field of an `AssetOptions` struct literal.
    ///
    /// The field name keeps its span, so the compiler reports unknown and repeated
    /// names, and editors complete and document them. A name without a colon is a
    /// shorthand field, as in Rust while the name is being typed, and the compiler
    /// reports the missing value. A colon without a value sets the field to `None`
    /// and reports the missing value at the colon.
    pub(crate) fn field(&self) -> TokenStream {
        let field = self.field_name();
        let Some(colon_token) = self.colon_token else {
            return quote! { #field };
        };
        if self.value.is_empty() {
            let error = syn::Error::new(
                colon_token.span,
                format!("expected a value for `{}`", self.name()),
            )
            .to_compile_error();
            return quote! { #field: { #error; ::core::option::Option::None } };
        }

        let value = &self.value;
        let error = self.checksum_error().map(|error| error.to_compile_error());
        quote! {
            #field: {
                #error
                ::core::option::Option::Some(::std::borrow::Cow::Borrowed(#value))
            }
        }
    }

    /// Checks that a literal checksum is written as `sha256:` followed by 64
    /// lowercase hex digits. Other values are checked when bundling.
    fn checksum_error(&self) -> Option<syn::Error> {
        if self.name() != "checksum" {
            return None;
        }
        let lit = syn::parse2::<LitStr>(self.value.tokens.clone()).ok()?;
        let valid = lit.value().strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        (!valid).then(|| {
            syn::Error::new(
                lit.span(),
                "expected a checksum written as `\"sha256:<64 lowercase hex digits>\"`",
            )
        })
    }

    /// The name as written, with keywords escaped to raw identifiers to keep the
    /// expansion parseable.
    fn field_name(&self) -> Ident {
        match syn::parse_str::<Ident>(&self.name()) {
            Ok(_) => self.name.clone(),
            Err(_) => Ident::new_raw(&self.name(), self.name.span()),
        }
    }
}

impl Parse for AssetOption {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Keywords are accepted as names so that a partially typed name that
        // happens to be one keeps the call parsing.
        let name = input.call(Ident::parse_any)?;
        let colon_token: Option<Token![:]> = input.parse()?;
        let value = match colon_token {
            Some(_) => input.parse()?,
            None => lenient::Expr {
                tokens: TokenStream::new(),
                span: input.span(),
            },
        };
        Ok(Self {
            name,
            colon_token,
            value,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for AssetOption {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.name.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        if !self.value.is_empty() {
            " ".pretty_print(printer);
            self.value.pretty_print(printer);
        }
    }
}
