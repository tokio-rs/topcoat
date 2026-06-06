use std::collections::HashMap;

use crate::runtime::{Attribute, AttributeValueViewParts, AttributeViewParts, ViewPart, ViewParts};

/// A runtime collection of HTML attributes with unique keys.
///
/// Prefer constructing `Attributes` with the
/// [`attributes!`](https://docs.rs/topcoat/latest/topcoat/view/macro.attributes.html)
/// macro. The macro accepts the same attribute syntax as an element inside
/// `view!`, including dynamic values, dynamic names, event handlers, and
/// attribute-level control flow.
///
/// ```rust,ignore
/// use topcoat::view::{attributes, view};
///
/// let attrs = attributes! {
///     class="button"
///     type="submit"
///     aria-label="Save changes"
/// };
///
/// view! {
///     <button (attrs)>"Save"</button>
/// }
/// ```
///
/// `Attributes` is map-like: each key appears at most once, and inserting the
/// same key again replaces the previous value. Do not rely on render order.
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
    pub fn new() -> Self {
        Default::default()
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

    /// Returns an iterator over attribute keys and rendered values.
    #[inline]
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
