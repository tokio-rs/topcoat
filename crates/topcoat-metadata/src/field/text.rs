use std::borrow::Cow;

use topcoat_core::context::Cx;
#[cfg(feature = "l10n")]
use topcoat_l10n::Translation;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

/// A text value used in page metadata, such as a title or description.
///
/// Convert a string into this type, or use a [`Translation`] when the `l10n`
/// feature is enabled. [`resolve`](Self::resolve) returns the text to use
/// for the current request.
#[derive(Clone)]
#[non_exhaustive]
pub enum Text {
    /// A string used unchanged across requests.
    Str(Cow<'static, str>),
    /// Text translated into the request's locale.
    #[cfg(feature = "l10n")]
    Translation(Translation<'static>),
}

impl Text {
    /// Returns the text using the locale in `cx` if translation is needed.
    #[must_use]
    #[cfg_attr(not(feature = "l10n"), allow(unused_variables))]
    pub fn resolve(&self, cx: &Cx) -> Cow<'_, str> {
        match self {
            Self::Str(text) => Cow::Borrowed(text),
            #[cfg(feature = "l10n")]
            Self::Translation(translation) => translation.resolve(cx),
        }
    }

    /// Adds the text to the view, translating it with `cx` if needed.
    #[cfg(feature = "view")]
    #[cfg_attr(not(feature = "l10n"), allow(unused_variables))]
    fn push(&self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Self::Str(Cow::Borrowed(text)) => parts.push_static_str(text),
            Self::Str(Cow::Owned(text)) => parts.push_str(text),
            #[cfg(feature = "l10n")]
            Self::Translation(translation) => match translation.resolve(cx) {
                Cow::Borrowed(text) => parts.push_static_str(text),
                Cow::Owned(text) => parts.push_string(text),
            },
        };
    }
}

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Self {
        Self::Str(Cow::Borrowed(text))
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self::Str(Cow::Owned(text))
    }
}

impl From<Cow<'static, str>> for Text {
    fn from(text: Cow<'static, str>) -> Self {
        Self::Str(text)
    }
}

#[cfg(feature = "l10n")]
impl From<Translation<'static>> for Text {
    fn from(translation: Translation<'static>) -> Self {
        Self::Translation(translation)
    }
}

/// Renders escaped text using the view's context to resolve translations.
#[cfg(feature = "view")]
impl NodeViewParts for &Text {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}

/// Renders text in an HTML attribute, resolving translations with the view's
/// context. For example, `<meta name="description" content=(description)>`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for &Text {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}
