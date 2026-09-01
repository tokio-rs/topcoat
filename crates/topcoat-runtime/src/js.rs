use std::borrow::Cow;

use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, PartsWriter};

/// The JavaScript source of a runtime expression, captured at its `$(..)`
/// site.
///
/// The `expr!` macro builds one of these next to the expression's Rust
/// value: the source as it reaches the browser, with the values captured
/// from the surrounding Rust scope serialized in place. Nothing is written
/// until the expression is spliced into a view, where the source renders
/// inside a marker comment.
#[derive(Debug, Clone)]
pub struct Js {
    parts: Vec<JsPart>,
}

#[derive(Debug, Clone)]
enum JsPart {
    /// Source text, escaped for the position it renders in.
    Source(Cow<'static, str>),
    /// Trusted scaffolding written verbatim, like the `const [..] = [..]`
    /// wrapper around captured values.
    Raw(&'static str),
    /// A captured value as JSON, hydrated on the client.
    Surrogate(String),
}

impl Js {
    /// Source without captured values.
    #[must_use]
    pub fn source(js: impl Into<Cow<'static, str>>) -> Self {
        Self {
            parts: vec![JsPart::Source(js.into())],
        }
    }

    /// Starts source that interleaves captured values.
    #[must_use]
    pub fn builder() -> JsBuilder {
        JsBuilder { parts: Vec::new() }
    }

    /// Writes the source through `parts`, sealed for the writer's current
    /// context: a marker comment's body or a double-quoted attribute value.
    pub(crate) fn write(&self, parts: &mut PartsWriter<'_>) {
        for part in &self.parts {
            match part {
                JsPart::Source(Cow::Borrowed(js)) => {
                    parts.push_static_str(js);
                }
                JsPart::Source(Cow::Owned(js)) => {
                    parts.push_str(js);
                }
                JsPart::Raw(js) => {
                    parts.push_static_str_unescaped(js);
                }
                JsPart::Surrogate(json) => {
                    parts
                        .push_promoted_str_unescaped(&"cx.hydrate(")
                        .push_str(json)
                        .push_promoted_str_unescaped(&")");
                }
            }
        }
    }
}

/// The source as the value of a `data-topcoat-*` attribute, like an event
/// handler or a bind expression.
impl AttributeValueViewParts for Js {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.write(parts);
    }
}

/// Builds a [`Js`] from source and captured values, in order.
#[derive(Debug)]
pub struct JsBuilder {
    parts: Vec<JsPart>,
}

impl JsBuilder {
    /// Appends source text.
    #[must_use]
    pub fn source(mut self, js: impl Into<Cow<'static, str>>) -> Self {
        self.parts.push(JsPart::Source(js.into()));
        self
    }

    /// Appends trusted scaffolding, written verbatim.
    #[must_use]
    pub fn raw(mut self, js: &'static str) -> Self {
        self.parts.push(JsPart::Raw(js));
        self
    }

    /// Appends a captured value, serialized now so the Rust expression can
    /// consume it afterwards.
    ///
    /// # Panics
    ///
    /// Panics if the value fails to serialize.
    #[must_use]
    pub fn surrogate(mut self, value: &(impl Serialize + ?Sized)) -> Self {
        let json = serde_json::to_string(value).expect("failed to serialize surrogate value");
        self.parts.push(JsPart::Surrogate(json));
        self
    }

    /// Finishes the builder into a [`Js`].
    #[must_use]
    pub fn build(self) -> Js {
        Js { parts: self.parts }
    }
}
