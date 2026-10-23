use syn::{
    Attribute, Meta,
    parse::{Parse, ParseStream},
};

/// Optional documentation written as `///` comments or `#[doc = "..."]` attributes.
///
/// Rejects all other attributes.
pub struct DocComments {
    /// Parsed `doc` attributes, with one attribute for each comment line.
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

/// Keeps each doc comment on its own line before the item it documents.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for DocComments {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.attrs.pretty_print(printer);
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
        assert_eq!(parse("").unwrap().attrs.len(), 0);
    }

    #[test]
    fn rejects_other_attributes() {
        for input in ["#[allow(unused)]", "#[doc(hidden)]", "/// One.\n#[inline]"] {
            assert!(parse(input).is_err(), "{input}");
        }
    }
}
