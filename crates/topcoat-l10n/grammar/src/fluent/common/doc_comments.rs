use syn::{
    Attribute, Meta,
    parse::{Parse, ParseStream},
};

/// Doc comments, written as `///` or `#[doc = "..."]`, or nothing.
///
/// Any other attribute is an error.
pub struct DocComments {
    /// The `doc` attributes, one per line of a `///` comment.
    pub attrs: Vec<Attribute>,
}

impl Parse for DocComments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        for attr in &attrs {
            if !matches!(&attr.meta, Meta::NameValue(meta) if meta.path.is_ident("doc")) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "only doc comments are allowed here",
                ));
            }
        }
        Ok(Self { attrs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<DocComments> {
        syn::parse_str(input)
    }

    #[test]
    fn accepts_doc_comments() {
        assert_eq!(parse("/// One.\n/// Two.").unwrap().attrs.len(), 2);
        assert_eq!(parse(r#"#[doc = "One."]"#).unwrap().attrs.len(), 1);
        assert!(parse("").unwrap().attrs.is_empty());
    }

    #[test]
    fn rejects_other_attributes() {
        for input in ["#[allow(unused)]", "#[doc(hidden)]", "/// One.\n#[inline]"] {
            assert!(parse(input).is_err(), "{input}");
        }
    }
}
