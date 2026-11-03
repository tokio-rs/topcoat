use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::common::{ParseOption, lenient, paths::topcoat_font};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(tech);
}

/// A `tech(...)` hint on a CSS `@font-face` `src` entry.
///
/// Accepts a CSS keyword string such as `"color-colrv1"` or an expression that
/// evaluates to [`topcoat_font::FontTech`].
pub struct FontTechHint {
    pub tech_kw: kw::tech,
    pub paren_token: Paren,
    pub value: FontTech,
}

impl Parse for FontTechHint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            tech_kw: input.parse()?,
            paren_token: parenthesized!(content in input),
            value: content.parse()?,
        })
    }
}

impl ParseOption for FontTechHint {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::tech)
    }
}

impl ToTokens for FontTechHint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontTechHint {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::common::pretty::Delim;

        printer.move_cursor(self.tech_kw.span().start());
        "tech".pretty_print(printer);
        self.paren_token.pretty_print(printer, None, |printer| {
            self.value.pretty_print(printer);
        });
    }
}

/// The technology inside a [`FontTechHint`].
///
/// A string literal is a CSS keyword, and an unknown keyword expands to a
/// compile error. Other expressions must evaluate to
/// [`topcoat_font::FontTech`].
pub struct FontTech(pub lenient::Expr);

impl Parse for FontTech {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for FontTech {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Ok(keyword) = syn::parse2::<LitStr>(self.0.tokens.clone()) else {
            self.0.to_tokens(tokens);
            return;
        };
        match tech_variant(&keyword.value()) {
            Some(name) => {
                let variant = proc_macro2::Ident::new(name, keyword.span());
                quote! { #topcoat_font::FontTech::#variant }.to_tokens(tokens);
            }
            None => syn::Error::new_spanned(
                &keyword,
                format!("`{}` is not a valid font technology", keyword.value()),
            )
            .to_compile_error()
            .to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for FontTech {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}

/// Returns the `FontTech` variant for a CSS keyword, or `None` if unknown.
fn tech_variant(keyword: &str) -> Option<&'static str> {
    Some(match keyword {
        "color-cbdt" => "ColorCbdt",
        "color-colrv0" => "ColorColrV0",
        "color-colrv1" => "ColorColrV1",
        "color-sbix" => "ColorSbix",
        "color-svg" => "ColorSvg",
        "features-aat" => "FeaturesAat",
        "features-graphite" => "FeaturesGraphite",
        "features-opentype" => "FeaturesOpenType",
        "incremental" => "Incremental",
        "palettes" => "Palettes",
        "variations" => "Variations",
        _ => return None,
    })
}
