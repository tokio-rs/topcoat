use std::collections::HashMap;

use topcoat_core::context::Cx;

use crate::{
    Attribute, AttributeCollector, AttributeKey, AttributeKeyViewParts, AttributeValue,
    AttributeValueViewParts, AttributeViewParts, HtmlContext, PartsWriter,
};

/// A collection of HTML attributes with unique keys, built at run time.
///
/// `Attributes` works like a map: each key appears at most once, and
/// inserting a key again replaces its value. The order in which attributes
/// render is unspecified. Build a collection with the
/// [`attributes!`](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html)
/// macro, and render it in the attribute position of an element.
///
/// Keys and values are captured as an [`AttributeKey`] and an
/// [`AttributeValue`] when they are inserted, so a collection does not
/// borrow from the values it was built from.
#[derive(Debug, Default, Clone)]
pub struct Attributes {
    map: HashMap<AttributeKey, AttributeValue>,
}

impl Attributes {
    /// Creates an empty collection.
    ///
    /// The
    /// [`attributes!`](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html)
    /// macro is usually more convenient. Use this constructor to fill a
    /// collection step by step.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Attributes::default()
    }

    /// Creates an empty collection with space for at least `capacity`
    /// attributes.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }

    /// Returns `true` if the collection contains the key `k`.
    ///
    /// A key whose value is [absent](AttributeValue::Absent) still counts.
    #[inline]
    pub fn contains_key(&self, k: impl AsRef<str>) -> bool {
        self.map.contains_key(k.as_ref())
    }

    /// Returns the value stored for the key `k`, if any.
    #[inline]
    pub fn get(&self, k: impl AsRef<str>) -> Option<&AttributeValue> {
        self.map.get(k.as_ref())
    }

    /// Inserts an attribute, returning the previous value for the key if
    /// there was one.
    ///
    /// The key is captured as an [`AttributeKey`] and the value as an
    /// [`AttributeValue`]. A value that leaves out its attribute, such as
    /// `false` or `None`, is stored as [absent](AttributeValue::Absent). It
    /// still replaces the previous value, but the attribute does not render.
    #[inline]
    pub fn insert(
        &mut self,
        cx: &Cx,
        k: impl AttributeKeyViewParts,
        v: impl AttributeValueViewParts,
    ) -> Option<AttributeValue> {
        let mut collector = AttributeCollector::new();
        k.into_view_parts(
            cx,
            &mut PartsWriter::collecting(&mut collector, cx, HtmlContext::AttributeKey),
        );
        let key = collector.finish(cx);
        let value = if v.attribute_present() {
            let mut collector = AttributeCollector::new();
            v.into_view_parts(
                cx,
                &mut PartsWriter::collecting(&mut collector, cx, HtmlContext::AttributeValue),
            );
            collector.finish(cx)
        } else {
            AttributeValue::Absent
        };
        self.map.insert(key, value)
    }

    /// Removes the key `k`, returning its value if it was present.
    #[inline]
    pub fn remove(&mut self, k: impl AsRef<str>) -> Option<AttributeValue> {
        self.map.remove(k.as_ref())
    }

    /// Removes all attributes from the collection.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Inserts every `(key, value)` entry from `iter`, replacing the values
    /// of keys that are already present.
    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = (AttributeKey, AttributeValue)>) {
        self.map.extend(iter);
    }

    /// Returns an iterator over the keys and values, in unspecified order.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl AttributeViewParts for Attributes {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        for (key, value) in self {
            Attribute::new(key, value).into_view_parts(cx, parts);
        }
    }
}

impl IntoIterator for Attributes {
    type Item = (AttributeKey, AttributeValue);
    type IntoIter = std::collections::hash_map::IntoIter<AttributeKey, AttributeValue>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a AttributeKey, &'a AttributeValue);
    type IntoIter = std::collections::hash_map::Iter<'a, AttributeKey, AttributeValue>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use topcoat_core::context::Cx;

    use super::*;
    use crate::internal::Builder;

    fn render(cx: &Cx, attrs: Attributes) -> String {
        Builder::build(cx, |b| b.attributes(attrs)).render(cx)
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
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        assert!(attrs.contains_key("class"));
        assert!(!attrs.contains_key("id"));
    }

    #[test]
    fn insert_returns_none_for_new_key() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        assert!(attrs.insert(cx, "class", "button").is_none());
    }

    #[test]
    fn insert_replaces_existing_value() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        let previous = attrs.insert(cx, "class", "link");
        assert!(previous.is_some());
        assert_eq!(render(cx, attrs), " class=\"link\"");
    }

    #[test]
    fn get_returns_inserted_value() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        assert!(attrs.get("class").is_some());
        assert!(attrs.get("missing").is_none());
    }

    #[test]
    fn remove_returns_value_and_deletes_entry() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        assert!(attrs.remove("class").is_some());
        assert!(!attrs.contains_key("class"));
        assert!(attrs.remove("class").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        attrs.insert(cx, "id", "submit");
        attrs.clear();
        assert_eq!(attrs.iter().count(), 0);
        assert!(!attrs.contains_key("class"));
    }

    #[test]
    fn renders_single_attribute() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        assert_eq!(render(cx, attrs), " class=\"button\"");
    }

    #[test]
    fn renders_multiple_attributes() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        attrs.insert(cx, "id", "submit");
        let rendered = render(cx, attrs);
        let parts: HashSet<&str> = rendered
            .split_terminator(' ')
            .filter(|s| !s.is_empty())
            .collect();
        let expected: HashSet<&str> = ["class=\"button\"", "id=\"submit\""].into_iter().collect();
        assert_eq!(parts, expected);
    }

    #[test]
    fn escapes_attribute_value() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "data-x", "a\"b<c");
        assert_eq!(render(cx, attrs), " data-x=\"a&quot;b<c\"");
    }

    #[test]
    fn omits_false_boolean_attribute() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "disabled", false);
        assert_eq!(render(cx, attrs), "");
    }

    #[test]
    fn renders_true_boolean_attribute() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "disabled", true);
        assert_eq!(render(cx, attrs), " disabled=\"\"");
    }

    #[test]
    fn omits_none_option_attribute() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "title", Option::<&str>::None);
        assert!(!attrs.get("title").unwrap().is_present());
        assert_eq!(render(cx, attrs), "");
    }

    #[test]
    fn renders_some_option_attribute() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "title", Some("hello"));
        assert_eq!(render(cx, attrs), " title=\"hello\"");
    }

    #[test]
    fn iter_yields_inserted_entries() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        attrs.insert(cx, "id", "submit");
        let keys: HashSet<&str> = attrs.iter().map(|(k, _)| k.as_str()).collect();
        let expected: HashSet<&str> = ["class", "id"].into_iter().collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn into_iter_yields_inserted_entries() {
        let cx = &Cx::default();
        let mut attrs = Attributes::new();
        attrs.insert(cx, "class", "button");
        attrs.insert(cx, "id", "submit");
        let keys: HashSet<String> = attrs
            .into_iter()
            .map(|(k, _)| k.as_str().to_owned())
            .collect();
        let expected: HashSet<String> = ["class", "id"].into_iter().map(String::from).collect();
        assert_eq!(keys, expected);
    }
}
