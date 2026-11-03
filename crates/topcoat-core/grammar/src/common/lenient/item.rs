use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use syn::{
    Attribute, Signature, Visibility,
    parse::{Parse, ParseStream},
};

use crate::common::lenient::Block;

/// A function item whose body is kept as written.
///
/// Like [`syn::ItemFn`], but the body is a [`Block`]. The attributes,
/// visibility and signature are parsed as usual, so the function's outline
/// can be inspected, while code that is still being typed in the body does not
/// stop the function from parsing.
#[derive(Clone)]
pub struct ItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub block: Block,
}

impl Parse for ItemFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            sig: input.parse()?,
            block: input.parse()?,
        })
    }
}

impl ToTokens for ItemFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(&self.attrs);
        self.vis.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        self.block.to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> String {
        source.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn parses_a_function_with_an_incomplete_body() {
        let source = "/// Docs.\n#[inline] pub async fn card<T>(title: &str) -> u8 where T: Copy { user. }";
        let item: ItemFn = syn::parse_str(source).unwrap();
        assert_eq!(item.attrs.len(), 2);
        assert_eq!(item.sig.ident, "card");
        assert_eq!(item.block.stmts.to_string(), tokens("user."));
        assert_eq!(item.to_token_stream().to_string(), tokens(source));
    }

    #[test]
    fn rejects_an_incomplete_signature() {
        assert!(syn::parse_str::<ItemFn>("fn card(title:) {}").is_err());
    }
}
