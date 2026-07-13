use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Expr, Token, braced,
    parse::{Parse, ParseStream},
    token::Brace,
};
use topcoat_core_grammar::paths::topcoat_font;

use crate::font_face::FontFace;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(face);
}

/// A `font!` invocation: a family name followed by its faces, given either as
/// CSS-like `@font-face` blocks or as a single expression.
pub struct Font {
    pub family: Expr,
    pub comma_token: Token![,],
    pub faces: FontFaces,
}

impl Parse for Font {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            family: input.parse()?,
            comma_token: input.parse()?,
            faces: input.parse()?,
        })
    }
}

impl ToTokens for Font {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let family = &self.family;

        let data = match &self.faces {
            FontFaces::Css(blocks) => {
                let faces = blocks
                    .iter()
                    .map(|block| block.face.to_tokens_with_family(family));
                quote! {
                    #topcoat_font::FontData::new(#family, ::std::vec![#(#faces),*])
                }
            }
            FontFaces::Expr(faces) => {
                quote! { #topcoat_font::FontData::new(#family, #faces) }
            }
        };

        quote! {{
            static FONT_DATA: ::std::sync::LazyLock<#topcoat_font::FontData> =
                ::std::sync::LazyLock::new(|| #data);
            const FONT: #topcoat_font::Font = #topcoat_font::Font::new(&FONT_DATA);
            #topcoat_font::register_font!(FONT);
            FONT
        }}
        .to_tokens(tokens);
    }
}

/// The faces of a [`Font`], written as CSS-like `@font-face` blocks or as a
/// single expression convertible into
/// [`FontFaces`](topcoat_font::FontFaces).
pub enum FontFaces {
    Css(Vec<FontFaceBlock>),
    Expr(Box<Expr>),
}

impl Parse for FontFaces {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![@]) {
            let mut blocks = Vec::new();
            while input.peek(Token![@]) {
                blocks.push(input.parse()?);
            }
            // Accept an optional trailing comma after the block list.
            let _: Option<Token![,]> = input.parse()?;
            Ok(Self::Css(blocks))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

/// A single `@font-face { ... }` block. Its body is a
/// [`FontFace`] whose `font-family` is supplied by the enclosing [`Font`]
/// rather than written in the block.
pub struct FontFaceBlock {
    pub at_token: Token![@],
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub face_kw: kw::face,
    pub brace_token: Brace,
    pub face: FontFace,
}

impl Parse for FontFaceBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let at_token = input.parse()?;
        let font_kw = input.parse()?;
        let dash_token = input.parse()?;
        let face_kw = input.parse()?;
        let brace_token = braced!(content in input);
        let face: FontFace = content.parse()?;

        if let Some(family) = &face.family {
            return Err(syn::Error::new_spanned(
                family,
                "`font-family` is set automatically by `font!` and must not be written here",
            ));
        }

        Ok(Self {
            at_token,
            font_kw,
            dash_token,
            face_kw,
            brace_token,
            face,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Font {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.family.pretty_print(printer);
        ",".pretty_print(printer);
        printer.scan_same_line_trivia();
        printer.scan_force_break();
        printer.scan_trivia(true, true);

        match &self.faces {
            FontFaces::Css(blocks) => {
                for (index, block) in blocks.iter().enumerate() {
                    block.pretty_print(printer);
                    if index < blocks.len() - 1 {
                        printer.scan_same_line_trivia();
                        printer.scan_force_break();
                        printer.scan_trivia(true, true);
                    }
                }
            }
            FontFaces::Expr(faces) => faces.pretty_print(printer),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FontFaceBlock {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::pretty::{BreakMode, Delim};

        printer.move_cursor(self.at_token.span().start());
        "@font-face".pretty_print(printer);
        printer.move_cursor(self.face_kw.span().end());
        " ".pretty_print(printer);
        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.face.pretty_print(printer);
            });
    }
}
