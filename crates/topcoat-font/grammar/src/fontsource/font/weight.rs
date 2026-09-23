use syn::{
    Token,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::fontsource::{
    font::List,
    font_face::{WeightKey, WeightValue},
};

/// A `weight:` argument containing one weight or a list, such as `[400, 700]`.
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
impl topcoat_core_grammar::pretty::PrettyPrint for Weight {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
