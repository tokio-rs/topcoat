use std::collections::HashMap;

use topcoat_core::context::Cx;

use crate::{
    Attribute, AttributeValue, AttributeValueViewParts, AttributeViewParts, PartsWriter,
    internal::{block, build_sync},
};

/// A runtime collection of HTML attributes with unique keys.
///
/// `Attributes` is map-like: each key appears at most once, and inserting the
/// same key again replaces the previous value. Do not rely on render order.
/// Prefer constructing `Attributes` with the [`attributes!`](macro.attributes.html)
/// macro.
///
/// Each value is captured as an [`AttributeValue`]: inside a `view!`
/// invocation it lands in the enclosing instruction buffer, and elsewhere it
/// carries a buffer of its own, so a collection can be built and rendered
/// anywhere.
#[derive(Debug, Default, Clone)]
pub struct Attributes {
    map: HashMap<String, AttributeValue>,
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

    /// Returns the captured value stored for attribute key `k`, if present.
    #[inline]
    pub fn get(&self, k: impl AsRef<str>) -> Option<&AttributeValue> {
        self.map.get(k.as_ref())
    }

    /// Inserts or replaces an attribute.
    ///
    /// The value is captured as an [`AttributeValue`] with
    /// [`AttributeValueViewParts`]. If the key was already present, the
    /// previous captured value is returned. If the implementation of
    /// [`AttributeValueViewParts`] for `v` signals that the attribute should
    /// not be present, an [absent](AttributeValue::absent) value is stored
    /// instead, which causes the previous value to be replaced and the
    /// attribute not to be rendered in a `view!`.
    #[inline]
    pub fn insert(
        &mut self,
        cx: &Cx,
        k: impl Into<String>,
        v: impl AttributeValueViewParts,
    ) -> Option<AttributeValue> {
        let value = if v.attribute_present() {
            // A present value is always captured as an instruction block,
            // even when it writes nothing (a `true` boolean), so it is
            // never mistaken for an absent attribute.
            AttributeValue::captured(build_sync(|| block(cx, |b| b.attribute_value(v))))
        } else {
            AttributeValue::absent()
        };
        self.map.insert(k.into(), value)
    }

    /// Removes an attribute, returning its captured value if the key was
    /// present.
    #[inline]
    pub fn remove(&mut self, k: impl AsRef<str>) -> Option<AttributeValue> {
        self.map.remove(k.as_ref())
    }

    /// Removes all attributes from the collection.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Inserts every `(key, value)` entry from `iter`, replacing any keys
    /// already present.
    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = (String, AttributeValue)>) {
        self.map.extend(iter);
    }

    /// Returns an iterator over attribute keys and captured values.
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
    type Item = (String, AttributeValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, AttributeValue>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = (&'a String, &'a AttributeValue);
    type IntoIter = std::collections::hash_map::Iter<'a, String, AttributeValue>;

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
    use crate::{buffer::ViewBufferScope, internal::build_sync};

    /// Runs `f` with a request context inside a fresh view scope.
    fn in_scope<R>(f: impl FnOnce(&Cx) -> R) -> R {
        ViewBufferScope::scope_sync(|| f(&Cx::default())).0
    }

    fn render(cx: &Cx, attrs: Attributes) -> String {
        build_sync(|| block(cx, |b| b.attributes(attrs))).render(cx)
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
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            assert!(attrs.contains_key("class"));
            assert!(!attrs.contains_key("id"));
        });
    }

    #[test]
    fn insert_returns_none_for_new_key() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            assert!(attrs.insert(cx, "class", "button").is_none());
        });
    }

    #[test]
    fn insert_replaces_existing_value() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            let previous = attrs.insert(cx, "class", "link");
            assert!(previous.is_some());
            assert_eq!(render(cx, attrs), " class=\"link\"");
        });
    }

    #[test]
    fn get_returns_inserted_value() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            assert!(attrs.get("class").is_some());
            assert!(attrs.get("missing").is_none());
        });
    }

    #[test]
    fn remove_returns_value_and_deletes_entry() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            assert!(attrs.remove("class").is_some());
            assert!(!attrs.contains_key("class"));
            assert!(attrs.remove("class").is_none());
        });
    }

    #[test]
    fn clear_removes_all_entries() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            attrs.insert(cx, "id", "submit");
            attrs.clear();
            assert_eq!(attrs.iter().count(), 0);
            assert!(!attrs.contains_key("class"));
        });
    }

    #[test]
    fn renders_single_attribute() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            assert_eq!(render(cx, attrs), " class=\"button\"");
        });
    }

    #[test]
    fn renders_multiple_attributes() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            attrs.insert(cx, "id", "submit");
            let rendered = render(cx, attrs);
            let parts: HashSet<&str> = rendered
                .split_terminator(' ')
                .filter(|s| !s.is_empty())
                .collect();
            let expected: HashSet<&str> =
                ["class=\"button\"", "id=\"submit\""].into_iter().collect();
            assert_eq!(parts, expected);
        });
    }

    #[test]
    fn escapes_attribute_value() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "data-x", "a\"b<c");
            assert_eq!(render(cx, attrs), " data-x=\"a&quot;b<c\"");
        });
    }

    #[test]
    fn omits_false_boolean_attribute() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "disabled", false);
            assert_eq!(render(cx, attrs), "");
        });
    }

    #[test]
    fn renders_true_boolean_attribute() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "disabled", true);
            assert_eq!(render(cx, attrs), " disabled=\"\"");
        });
    }

    #[test]
    fn omits_none_option_attribute() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "title", Option::<&str>::None);
            assert!(!attrs.get("title").unwrap().is_present());
            assert_eq!(render(cx, attrs), "");
        });
    }

    #[test]
    fn renders_some_option_attribute() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "title", Some("hello"));
            assert_eq!(render(cx, attrs), " title=\"hello\"");
        });
    }

    #[test]
    fn iter_yields_inserted_entries() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            attrs.insert(cx, "id", "submit");
            let keys: HashSet<&str> = attrs.iter().map(|(k, _)| k.as_str()).collect();
            let expected: HashSet<&str> = ["class", "id"].into_iter().collect();
            assert_eq!(keys, expected);
        });
    }

    #[test]
    fn into_iter_yields_inserted_entries() {
        in_scope(|cx| {
            let mut attrs = Attributes::new();
            attrs.insert(cx, "class", "button");
            attrs.insert(cx, "id", "submit");
            let keys: HashSet<String> = attrs.into_iter().map(|(k, _)| k).collect();
            let expected: HashSet<String> = ["class", "id"].into_iter().map(String::from).collect();
            assert_eq!(keys, expected);
        });
    }
}
