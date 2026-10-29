use std::borrow::Cow;

use topcoat_core::context::Cx;
#[cfg(feature = "l10n")]
use topcoat_l10n::Translation;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

/// Text in a page's metadata, such as its title or description.
///
/// Create it from a string, or from a [`Translation`] with the `l10n`
/// feature. Call [`resolve`](Self::resolve) to get the text for a request.
#[derive(Clone)]
#[non_exhaustive]
pub enum Text {
    /// Text that is the same for every request.
    Str(Cow<'static, str>),
    /// A translation, resolved in the locale of the request.
    #[cfg(feature = "l10n")]
    Translation(Translation<'static>),
}

impl Text {
    /// Returns the text for the request of `cx`.
    #[must_use]
    #[cfg_attr(not(feature = "l10n"), allow(unused_variables))]
    pub fn resolve(&self, cx: &Cx) -> Cow<'_, str> {
        match self {
            Self::Str(text) => Cow::Borrowed(text),
            #[cfg(feature = "l10n")]
            Self::Translation(translation) => translation.resolve(cx),
        }
    }

    /// Resolves the text for the request of `cx` and appends it to the view.
    #[cfg(feature = "view")]
    #[cfg_attr(not(feature = "l10n"), allow(unused_variables))]
    fn push(&self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Self::Str(Cow::Borrowed(text)) => parts.push_static_str(*text),
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

/// Displays the text, resolved for the view's context, as escaped content.
#[cfg(feature = "view")]
impl NodeViewParts for &Text {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}

/// Displays the text, resolved for the view's context, in an attribute
/// value, as in `<meta name="description" content=(description)>`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for &Text {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}
