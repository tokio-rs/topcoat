use std::collections::HashMap;

use crate::runtime::{Attribute, AttributeValueViewParts, AttributeViewParts, ViewPart, ViewParts};

/// A runtime collection of HTML attributes with unique keys.
///
/// `Attributes` is map-like: each key appears at most once, and inserting the
/// same key again replaces the previous value. Do not rely on render order.
/// Prefer constructing `Attributes` with the [`attributes!`](macro.attributes.html)
/// macro.
#[derive(Debug, Default, Clone)]
pub struct Attributes {
    map: HashMap<String, ViewPart>,
}

impl Attributes {
    /// Creates an empty attribute collection.
    ///
    /// Prefer the
    /// [`attributes!`](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html)
    /// macro when writing attributes directly. Use this constructor when the
    /// collection must be populated incrementally.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Attributes::default()
    }

    /// Creates an empty attribute collection with space for at least `capacity`
    /// attributes.
    ///
    /// Prefer the
    /// [`attributes!`](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html)
    /// macro when writing attributes directly. This is mainly useful for
    /// generated code or manual builders that already know how many attributes
    /// they will insert.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }

    /// Returns `true` if this collection contains an attribute with key `k`.
    #[inline]
    pub fn contains_key(&self, k: impl AsRef<str>) -> bool {
        self.map.contains_key(k.as_ref())
    }

    /// Returns the view parts stored for attribute key `k`, if present.
    #[inline]
    pub fn get(&mut self, k: impl AsRef<str>) -> Option<&ViewPart> {
        self.map.get(k.as_ref())
    }

    /// Inserts or replaces an attribute.
    ///
    /// The value is converted with [`AttributeValueViewParts`]. If the key was
    /// already present, the previous rendered value is returned.
    #[inline]
    pub fn insert(
        &mut self,
        k: impl Into<String>,
        v: impl AttributeValueViewParts,
    ) -> Option<ViewPart> {
        let mut view_parts = ViewParts::new();
        v.into_view_parts(&mut view_parts);
        self.map.insert(k.into(), view_parts.into())
    }

    /// Removes all attributes from the collection.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Inserts every `(key, value)` entry from `iter`, replacing any keys
    /// already present.
    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = (String, ViewPart)>) {
        self.map.extend(iter);
    }

    /// Returns an iterator over attribute keys and rendered values.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl AttributeViewParts for Attributes {
    fn into_view_parts(self, parts: &mut ViewParts) {
        for (key, value) in self {
            Attribute::new(key, value).into_view_parts(parts);
        }
    }
}

impl IntoIterator for Attributes {
    type Item = (String, ViewPart);
    type IntoIter = std::collections::hash_map::IntoIter<String, ViewPart>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a String, &'a ViewPart);
    type IntoIter = std::collections::hash_map::Iter<'a, String, ViewPart>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use topcoat_core::runtime::context::Cx;

    use super::*;
    use crate::runtime::{FmtHtml, Formatter};

    fn render(attrs: Attributes) -> String {
        let mut parts = ViewParts::new();
        attrs.into_view_parts(&mut parts);
        let part: ViewPart = parts.into();
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        part.fmt_html(&Cx::default(), &mut f);
        buf
    }

    #[test]
    fn new_is_empty() {
        let attrs = Attributes::new();
        assert!(!attrs.contains_key("class"));
        assert_eq!(attrs.iter().count(), 0);
    }

    #[test]
    fn with_capacity_is_empty() {
        let attrs = Attributes::with_capacity(4);
        assert_eq!(attrs.iter().count(), 0);
    }

    #[test]
    fn insert_then_contains_key() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        assert!(attrs.contains_key("class"));
        assert!(!attrs.contains_key("id"));
    }

    #[test]
    fn insert_returns_none_for_new_key() {
        let mut attrs = Attributes::new();
        assert!(attrs.insert("class", "button").is_none());
    }

    #[test]
    fn insert_replaces_existing_value() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        let previous = attrs.insert("class", "link");
        assert!(previous.is_some());
        assert_eq!(render(attrs), " class=\"link\"");
    }

    #[test]
    fn get_returns_inserted_value() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        assert!(attrs.get("class").is_some());
        assert!(attrs.get("missing").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        attrs.insert("id", "submit");
        attrs.clear();
        assert_eq!(attrs.iter().count(), 0);
        assert!(!attrs.contains_key("class"));
    }

    #[test]
    fn renders_single_attribute() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        assert_eq!(render(attrs), " class=\"button\"");
    }

    #[test]
    fn renders_multiple_attributes() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        attrs.insert("id", "submit");
        let rendered = render(attrs);
        let parts: HashSet<&str> = rendered
            .split_terminator(' ')
            .filter(|s| !s.is_empty())
            .collect();
        let expected: HashSet<&str> = ["class=\"button\"", "id=\"submit\""].into_iter().collect();
        assert_eq!(parts, expected);
    }

    #[test]
    fn escapes_attribute_value() {
        let mut attrs = Attributes::new();
        attrs.insert("data-x", "a\"b<c");
        assert_eq!(render(attrs), " data-x=\"a&quot;b&lt;c\"");
    }

    #[test]
    fn omits_false_boolean_attribute() {
        let mut attrs = Attributes::new();
        attrs.insert("disabled", false);
        assert_eq!(render(attrs), "");
    }

    #[test]
    fn renders_true_boolean_attribute() {
        let mut attrs = Attributes::new();
        attrs.insert("disabled", true);
        assert_eq!(render(attrs), " disabled=\"true\"");
    }

    #[test]
    fn omits_none_option_attribute() {
        let mut attrs = Attributes::new();
        attrs.insert("title", Option::<&str>::None);
        assert_eq!(render(attrs), "");
    }

    #[test]
    fn renders_some_option_attribute() {
        let mut attrs = Attributes::new();
        attrs.insert("title", Some("hello"));
        assert_eq!(render(attrs), " title=\"hello\"");
    }

    #[test]
    fn iter_yields_inserted_entries() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        attrs.insert("id", "submit");
        let keys: HashSet<&str> = attrs.iter().map(|(k, _)| k.as_str()).collect();
        let expected: HashSet<&str> = ["class", "id"].into_iter().collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn into_iter_yields_inserted_entries() {
        let mut attrs = Attributes::new();
        attrs.insert("class", "button");
        attrs.insert("id", "submit");
        let keys: HashSet<String> = attrs.into_iter().map(|(k, _)| k).collect();
        let expected: HashSet<String> = ["class", "id"].into_iter().map(String::from).collect();
        assert_eq!(keys, expected);
    }
}
