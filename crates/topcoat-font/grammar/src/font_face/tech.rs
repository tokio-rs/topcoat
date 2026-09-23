use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Lit, parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_font};

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
impl topcoat_core_grammar::pretty::PrettyPrint for FontTechHint {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::pretty::Delim;

        printer.move_cursor(self.tech_kw.span().start());
        "tech".pretty_print(printer);
        self.paren_token.pretty_print(printer, None, |printer| {
            self.value.pretty_print(printer);
        });
    }
}

/// The technology inside a [`FontTechHint`].
///
/// String literals are validated as CSS keywords during parsing. Other
/// expressions must evaluate to [`topcoat_font::FontTech`].
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
            quote! { #topcoat_font::FontTech::#variant }.to_tokens(tokens);
            return;
        }
        let inner = &self.0;
        inner.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontTech {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
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
