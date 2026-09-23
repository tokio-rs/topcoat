use http::HeaderValue;

/// How [`PatchElements`](crate::PatchElements) applies elements to the DOM.
///
/// Mirrors the modes accepted by the `datastar-patch-elements` event and the
/// `datastar-mode` response header.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ElementPatchMode {
    /// Morph the entire element into the existing element. The default.
    #[default]
    Outer,
    /// Morph the inner HTML of the existing element.
    Inner,
    /// Remove the existing element.
    Remove,
    /// Replace the existing element with the new element.
    Replace,
    /// Insert the element as the first child of the existing element.
    Prepend,
    /// Insert the element as the last child of the existing element.
    Append,
    /// Insert the element before the existing element.
    Before,
    /// Insert the element after the existing element.
    After,
}

impl ElementPatchMode {
    /// Returns the Datastar string for this patch mode (e.g. `"outer"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Outer => "outer",
            Self::Inner => "inner",
            Self::Remove => "remove",
            Self::Replace => "replace",
            Self::Prepend => "prepend",
            Self::Append => "append",
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

impl From<ElementPatchMode> for HeaderValue {
    fn from(mode: ElementPatchMode) -> Self {
        HeaderValue::from_static(mode.as_str())
    }
}
