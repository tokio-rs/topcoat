use std::borrow::Cow;

use serde::Serialize;
use topcoat_core::context::Cx;
use topcoat_view::{AttributeValueViewParts, PartsWriter};

/// JavaScript source with captured values, ready to render into a view.
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

    /// Returns executable JavaScript, including serialized captured values.
    ///
    /// The source expects the runtime context to be available as `cx`. It is
    /// not escaped for embedding in HTML.
    #[must_use]
    pub fn to_source(&self) -> String {
        let mut source = String::new();
        for part in &self.parts {
            match part {
                JsPart::Source(js) => source.push_str(js),
                JsPart::Raw(js) => source.push_str(js),
                JsPart::Surrogate(json) => {
                    source.push_str("cx.hydrate(");
                    source.push_str(json);
                    source.push(')');
                }
            }
        }
        source
    }

    /// Writes the source with escaping for the writer's current HTML context.
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

/// Renders the source as an HTML attribute value.
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
    /// Appends an expression in parentheses, preserving its captured values.
    #[must_use]
    pub fn expression<T>(mut self, expression: &crate::Expr<T>) -> Self {
        self.parts.push(JsPart::Raw("("));
        self.parts.extend(expression.js.parts.iter().cloned());
        self.parts.push(JsPart::Raw(")"));
        self
    }

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

    /// Serializes and appends a captured value.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_source_preserves_javascript_and_capture_escaping() {
        let js = Js::builder()
            .raw("(() => { const value = ")
            .surrogate(&"\"<>&\n")
            .source("; return value; })()")
            .build();

        assert_eq!(
            js.to_source(),
            r#"(() => { const value = cx.hydrate("\"<>&\n"); return value; })()"#,
        );
    }
}
