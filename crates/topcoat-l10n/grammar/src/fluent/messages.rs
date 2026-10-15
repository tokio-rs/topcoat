use syn::{
    Visibility,
    parse::{Parse, ParseStream},
};

use crate::fluent::PathPattern;

/// The input of a `messages!` invocation: an optional visibility followed by
/// the path of the Fluent files to load.
///
/// ```text
/// messages!(pub(crate) "./l10n/{locale}.ftl")
/// ```
pub struct Messages {
    /// The visibility of the generated module.
    pub vis: Visibility,
    /// The path of the Fluent files.
    pub path: PathPattern,
}

impl Parse for Messages {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            path: input.parse()?,
        })
    }
}
