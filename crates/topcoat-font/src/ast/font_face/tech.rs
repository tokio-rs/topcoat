use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Lit, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(tech);
}

/// A `tech(...)` hint on a CSS `@font-face` `src` entry.
///
/// The technology may be a string literal naming a CSS technology keyword (such
/// as `"color-colrv1"`) or a parenthesized expression resolving to a
/// [`FontTech`] at run time.
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
impl topcoat_pretty::PrettyPrint for FontTechHint {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_pretty::Delim;

        printer.move_cursor(self.tech_kw.span().start());
        "tech".pretty_print(printer);
        self.paren_token.pretty_print(printer, None, |printer| {
            self.value.pretty_print(printer);
        });
    }
}

/// The technology inside a [`FontTechHint`].
///
/// Wraps an expression that resolves to a [`crate::runtime::FontTech`] at run
/// time. When the expression is a string literal, it is validated at compile
/// time against the known CSS technology keywords.
pub struct FontTech(pub Expr);

impl Parse for FontTech {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        if let Expr::Lit(lit) = &expr
            && let Lit::Str(keyword) = &lit.lit
            && tech_variant(&keyword.value()).is_none()
        {
            return Err(syn::Error::new_spanned(
                keyword,
                format!("`{}` is not a valid font technology", keyword.value()),
            ));
        }
        Ok(Self(expr))
    }
}

impl ToTokens for FontTech {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Expr::Lit(lit) = &self.0
            && let Lit::Str(keyword) = &lit.lit
        {
            let name = tech_variant(&keyword.value()).expect("validated at parse time");
            let variant = proc_macro2::Ident::new(name, keyword.span());
            quote! { ::topcoat::font::FontTech::#variant }.to_tokens(tokens);
            return;
        }
        let inner = &self.0;
        inner.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontTech {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.0.pretty_print(printer);
    }
}

/// The `FontTech` variant identifier for a CSS `tech(...)` keyword, or `None` if
/// the keyword names no known technology.
///
/// This single mapping backs both parse-time validation and codegen, so the two
/// can never disagree.
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
