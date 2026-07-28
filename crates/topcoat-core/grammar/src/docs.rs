use syn::Attribute;

/// The doc comments among `attrs`.
///
/// A macro that replaces the user's item with a generated one under the same
/// name carries these over, so the item callers see keeps the documentation
/// the item they wrote was given.
pub fn doc_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attr| attr.path().is_ident("doc"))
}
