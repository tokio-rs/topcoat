use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    font::List,
    font_face::{WeightKey, WeightValue},
};

/// A `weight:` argument for `fontsource_font!`: one weight or a bracketed list
/// of weights to cross-product, e.g. `weight: [400, 700]`.
pub struct Weight {
    pub key: WeightKey,
    pub colon_token: Token![:],
    pub value: List<WeightValue>,
}

impl Parse for Weight {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Weight {
    fn peek(input: ParseStream) -> bool {
        WeightKey::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for Weight {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
