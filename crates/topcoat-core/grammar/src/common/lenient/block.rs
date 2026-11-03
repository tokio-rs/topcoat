use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use syn::{
    Attribute, braced,
    parse::{Parse, ParseStream},
    token::Brace,
};

use crate::common::ParseOption;

/// A block of Rust statements in braces, such as a function body.
///
/// The braces mark where the block ends, so every token inside is kept as
/// written, even code that does not parse yet. The compiler checks the
/// statements once the block is expanded.
#[derive(Clone)]
pub struct Block {
    pub brace_token: Brace,
    /// The inner attributes at the start of the block, such as
    /// `#![allow(unused)]`. They apply to the item the block belongs to.
    pub inner_attrs: Vec<Attribute>,
    /// The tokens after the inner attributes.
    pub stmts: TokenStream,
}

impl Parse for Block {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            brace_token: braced!(content in input),
            inner_attrs: content.call(Attribute::parse_inner)?,
            stmts: content.parse()?,
        })
    }
}

impl ParseOption for Block {
    fn peek(input: ParseStream) -> bool {
        input.peek(Brace)
    }
}

impl ToTokens for Block {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            tokens.append_all(&self.inner_attrs);
            self.stmts.to_tokens(tokens);
        });
    }
}

/// Formats the statements one per line. A block whose statements do not
/// parse yet, or that has inner attributes, is copied as written.
#[cfg(feature = "pretty")]
impl crate::common::pretty::PrettyPrint for Block {
    fn pretty_print(&self, printer: &mut crate::common::pretty::Printer<'_>) {
        use syn::parse::Parser;

        let stmts = syn::Block::parse_within.parse2(self.stmts.clone());
        match stmts {
            Ok(stmts) if self.inner_attrs.is_empty() => syn::Block {
                brace_token: self.brace_token,
                stmts,
            }
            .pretty_print(printer),
            _ => syn::Expr::Verbatim(self.to_token_stream()).pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> String {
        source.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn keeps_every_token_inside_the_braces() {
        for source in ["", "let a = 1; a", "user.", "let x = ; f(,)"] {
            let block: Block = syn::parse_str(&format!("{{ {source} }}")).unwrap();
            assert_eq!(block.stmts.to_string(), tokens(source), "{source}");
        }
    }

    #[test]
    fn separates_inner_attributes_from_the_statements() {
        let source = "{ #![allow(unused)] #[inline] fn f() {} f() }";
        let block: Block = syn::parse_str(source).unwrap();
        assert_eq!(block.inner_attrs.len(), 1);
        assert_eq!(block.stmts.to_string(), tokens("#[inline] fn f() {} f()"));

        let expanded: Block = syn::parse2(block.to_token_stream()).unwrap();
        assert_eq!(expanded.inner_attrs.len(), 1);
        assert_eq!(expanded.stmts.to_string(), block.stmts.to_string());
    }

    #[test]
    fn requires_braces() {
        assert!(syn::parse_str::<Block>("(a)").is_err());
    }

    #[test]
    fn expands_to_the_block_as_written() {
        let block: Block = syn::parse_str("{ user. }").unwrap();
        assert_eq!(block.to_token_stream().to_string(), tokens("{ user. }"));
    }
}
