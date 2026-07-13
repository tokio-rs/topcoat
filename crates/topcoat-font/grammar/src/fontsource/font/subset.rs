use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;

use crate::fontsource::{
    font::List,
    font_face::{SubsetKey, SubsetValue},
};

/// A `subset:` argument for `fontsource_font!`: one subset or a bracketed list of
/// subsets to cross-product, e.g. `subset: [Subset::Latin, Subset::Cyrillic]`.
pub struct Subset {
    pub key: SubsetKey,
    pub colon_token: Token![:],
    pub value: List<SubsetValue>,
}

impl Parse for Subset {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for Subset {
    fn peek(input: ParseStream) -> bool {
        SubsetKey::peek(input)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Subset {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}
