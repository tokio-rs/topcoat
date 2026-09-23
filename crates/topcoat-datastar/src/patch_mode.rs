use http::HeaderValue;

/// How Datastar patches elements into the DOM.
///
/// The variants match the modes of the `datastar-patch-elements` event and
/// the `datastar-mode` response header. The default is
/// [`Outer`](Self::Outer).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ElementPatchMode {
    /// Morph the new element into the existing element, including the element
    /// itself.
    #[default]
    Outer,
    /// Morph the new element into the inner HTML of the existing element.
    Inner,
    /// Remove the existing element.
    Remove,
    /// Replace the existing element with the new element, without morphing.
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
    /// Returns the Datastar name of this mode, such as `"outer"`.
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
