use core::fmt;

#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};
use topcoat_core::context::Cx;

use crate::{
    AttributeCollector, CollectedPart, HtmlContext, HtmlWriter, RegionId, ViewHandle,
    buffer::ViewBuffer,
};

/// A view part that writes its output when the view renders, not when it is
/// built.
///
/// Implement this for values whose output is only known at render time,
/// such as resolved asset URLs, and push them with
/// [`PartsWriter::push_dyn`]. The writer passed to [`render`](Self::render)
/// carries the [`HtmlContext`] of the position the part was pushed into, so
/// everything written through it is escaped or validated for that position.
pub trait DynViewPart: 'static + fmt::Debug + Send + Sync {
    /// Writes this part's output into `w`.
    #[track_caller]
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>);

    /// Returns an estimate of the number of bytes this part will write.
    ///
    /// The estimate is used to pre-allocate the output, so aim for a close
    /// value. A slight over-estimate is usually better than an
    /// under-estimate. The default is `0`.
    #[inline]
    fn size_hint(&self) -> usize {
        0
    }
}

macro_rules! impl_push_primitive {
    ($method:ident, $ty:ty, $size_hint:expr) => {
        #[doc = concat!("Appends a `", stringify!($ty), "` rendered as text.")]
        ///
        /// The rendered number contains no character that is significant in
        /// any HTML context, so it is not escaped.
        #[inline]
        pub fn $method(&mut self, value: $ty) -> &mut Self {
            self.size_hint += $size_hint;
            self.sink.$method(value);
            self
        }
    };
}

/// The writer a value pushes its view parts into.
///
/// The `view!` macro hands a `PartsWriter` to the position trait that
/// matches each dynamic position in a template:
/// [`NodeViewParts`](crate::NodeViewParts),
/// [`AttributeValueViewParts`](crate::AttributeValueViewParts),
/// [`AttributeKeyViewParts`](crate::AttributeKeyViewParts),
/// [`ElementNameViewParts`](crate::ElementNameViewParts), or
/// [`AttributeViewParts`](crate::AttributeViewParts).
///
/// An implementation of those traits pushes the value through the `push_*`
/// methods, or delegates to another implementation of the same trait. Each
/// writer carries the [`HtmlContext`] of its position, and the `push_*`
/// methods tag the pushed text with that context, so rendering escapes or
/// validates it for the position. The `push_*_unescaped` methods are the
/// only way to skip that step.
///
/// The writer also adds up a size hint: an estimate of how many bytes
/// everything pushed so far writes when rendered. It becomes the size hint of
/// the built view, which pre-allocates the output when the view renders.
pub struct PartsWriter<'a> {
    sink: Sink<'a>,
    context: HtmlContext,
    size_hint: usize,
}

impl<'a> PartsWriter<'a> {
    /// Creates a writer that seals everything pushed into it with `context`.
    #[inline]
    pub(super) fn new(buffer: &'a mut ViewBuffer, context: HtmlContext) -> Self {
        Self {
            sink: Sink::Buffer(buffer),
            context,
            size_hint: 0,
        }
    }

    /// Creates a writer sealing for `context` whose pushes are collected
    /// into `collector` instead of a buffer.
    #[inline]
    pub(crate) fn collecting(
        collector: &'a mut AttributeCollector,
        cx: &'a Cx,
        context: HtmlContext,
    ) -> Self {
        Self {
            sink: Sink::Collector { collector, cx },
            context,
            size_hint: 0,
        }
    }

    /// Returns the accumulated size hint of everything pushed so far.
    #[inline]
    pub(super) fn size_hint(&self) -> usize {
        self.size_hint
    }

    /// Runs `f` with this writer sealing for a different context, then
    /// restores the current context.
    ///
    /// In-crate compositions that span more than one position use this to
    /// transition between the positions they cover, such as
    /// [`Attribute`](crate::Attribute) moving from a key to a value or
    /// [`push_comment`](Self::push_comment) sealing a comment body.
    ///
    /// This method should remain private to avoid potential XSS footguns.
    #[inline]
    pub(crate) fn in_context<R>(
        &mut self,
        context: HtmlContext,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous = std::mem::replace(&mut self.context, context);
        let result = f(self);
        self.context = previous;
        result
    }

    /// Estimates the bytes `value` writes when rendered in `context`.
    fn str_size_hint(value: &str, context: HtmlContext) -> usize {
        match context {
            HtmlContext::Unescaped => value.len(),
            // Assume some characters escape into multi-byte sequences.
            _ => value.len() + value.len() / 8,
        }
    }

    /// Appends a borrowed string, escaped for this writer's context.
    #[inline]
    pub fn push_str(&mut self, value: &str) -> &mut Self {
        self.size_hint += Self::str_size_hint(value, self.context);
        self.sink.push_str(value, self.context);
        self
    }

    /// Appends a static string, escaped for this writer's context.
    #[inline]
    pub fn push_static_str(&mut self, value: &'static str) -> &mut Self {
        self.size_hint += Self::str_size_hint(value, self.context);
        self.sink.push_static_str(value, self.context);
        self
    }

    /// Appends a string literal held by reference, escaped for this writer's
    /// context.
    ///
    /// Pass `&"..."`, which Rust promotes to a `&'static &'static str`. This
    /// is cheaper than [`push_static_str`](Self::push_static_str), so prefer
    /// it whenever the string is a literal.
    #[inline]
    pub fn push_promoted_str(&mut self, value: &'static &'static str) -> &mut Self {
        self.size_hint += Self::str_size_hint(value, self.context);
        self.sink.push_promoted_str(value, self.context);
        self
    }

    /// Appends an owned string, escaped for this writer's context.
    #[inline]
    pub fn push_string(&mut self, value: String) -> &mut Self {
        self.size_hint += Self::str_size_hint(&value, self.context);
        self.sink.push_string(value, self.context);
        self
    }

    /// Appends a borrowed string that renders verbatim, ignoring this
    /// writer's context.
    ///
    /// Use this only for trusted markup. Passing untrusted input skips
    /// escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_str_unescaped(&mut self, value: &str) -> &mut Self {
        self.size_hint += value.len();
        self.sink.push_str(value, HtmlContext::Unescaped);
        self
    }

    /// Appends a static string that renders verbatim, ignoring this writer's
    /// context.
    ///
    /// Use this only for trusted markup. Passing untrusted input skips
    /// escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_static_str_unescaped(&mut self, value: &'static str) -> &mut Self {
        self.size_hint += value.len();
        self.sink.push_static_str(value, HtmlContext::Unescaped);
        self
    }

    /// Appends a string literal held by reference that renders verbatim,
    /// ignoring this writer's context.
    ///
    /// Pass `&"..."`, which Rust promotes to a `&'static &'static str`. This
    /// is cheaper than
    /// [`push_static_str_unescaped`](Self::push_static_str_unescaped), so
    /// prefer it whenever the string is a literal.
    ///
    /// Use this only for trusted markup. Passing untrusted input skips
    /// escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_promoted_str_unescaped(&mut self, value: &'static &'static str) -> &mut Self {
        self.size_hint += value.len();
        self.sink.push_promoted_str(value, HtmlContext::Unescaped);
        self
    }

    /// Appends an owned string that renders verbatim, ignoring this writer's
    /// context.
    ///
    /// Use this only for trusted markup. Passing untrusted input skips
    /// escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_string_unescaped(&mut self, value: String) -> &mut Self {
        self.size_hint += value.len();
        self.sink.push_string(value, HtmlContext::Unescaped);
        self
    }

    /// Appends an HTML comment whose body is pushed by `build`.
    ///
    /// The `<!--` and `-->` delimiters are written verbatim. Everything
    /// `build` pushes through the writer it receives is escaped for the
    /// [`Comment`](HtmlContext::Comment) context. That context escapes `>`,
    /// so the body can never contain `-->` and end the comment early. This
    /// makes it safe to build a comment from untrusted data with
    /// [`push_str`](Self::push_str).
    ///
    /// # Panics
    ///
    /// Panics if this writer's context is not [`Text`](HtmlContext::Text).
    #[inline]
    pub fn push_comment(&mut self, build: impl FnOnce(&mut PartsWriter<'_>)) -> &mut Self {
        assert!(
            self.context == HtmlContext::Text,
            "tried to push comment in html context {:?}",
            self.context,
        );
        self.push_promoted_str_unescaped(&"<!--");
        self.in_context(HtmlContext::Comment, build);
        self.push_promoted_str_unescaped(&"-->");
        self
    }

    /// Appends a character, escaped for this writer's context.
    #[inline]
    pub fn push_char(&mut self, value: char) -> &mut Self {
        // One to four UTF-8 bytes, or an escape sequence.
        self.size_hint += 3;
        self.sink.push_char(value, self.context);
        self
    }

    // Each numeric size hint is the midpoint, rounded up, between the
    // shortest and widest output the type can render, including the leading
    // `-` for signed types (`isize`/`usize` assume a 64-bit target). A
    // float's rendered width is unbounded for extreme magnitudes, so the
    // upper end is the shortest round-trip form of a typical value.

    impl_push_primitive!(push_bool, bool, 5);
    impl_push_primitive!(push_i8, i8, 3);
    impl_push_primitive!(push_i16, i16, 4);
    impl_push_primitive!(push_i32, i32, 6);
    impl_push_primitive!(push_i64, i64, 11);
    impl_push_primitive!(push_i128, i128, 21);
    impl_push_primitive!(push_isize, isize, 11);
    impl_push_primitive!(push_u8, u8, 2);
    impl_push_primitive!(push_u16, u16, 3);
    impl_push_primitive!(push_u32, u32, 6);
    impl_push_primitive!(push_u64, u64, 11);
    impl_push_primitive!(push_u128, u128, 20);
    impl_push_primitive!(push_usize, usize, 11);
    impl_push_primitive!(push_f32, f32, 9);
    impl_push_primitive!(push_f64, f64, 13);

    /// Appends the start of the region `region`.
    ///
    /// # Panics
    ///
    /// Panics if used in a non-text HTML context.
    #[inline]
    pub(crate) fn push_region_start(&mut self, region: RegionId) -> &mut Self {
        assert!(
            self.context == HtmlContext::Text,
            "tried to push region start in html context {:?}",
            self.context,
        );
        self.size_hint += "<!--topcoat::region::start()-->".len() + 32;
        self.sink.push_region_start(region);
        self
    }

    /// Appends the end of the region `region`.
    ///
    /// # Panics
    ///
    /// Panics if used in a non-text HTML context.
    #[inline]
    pub(crate) fn push_region_end(&mut self, region: RegionId) -> &mut Self {
        assert!(
            self.context == HtmlContext::Text,
            "tried to push region end in html context {:?}",
            self.context,
        );
        self.size_hint += "<!--topcoat::region::end()-->".len() + 32;
        self.sink.push_region_end(region);
        self
    }

    /// Appends a part that writes its output at render time, escaped for this
    /// writer's context.
    #[inline]
    pub fn push_dyn(&mut self, part: Box<dyn DynViewPart>) -> &mut Self {
        self.size_hint += part.size_hint();
        self.sink.push_dyn(part, self.context);
        self
    }

    /// Appends a nested view.
    ///
    /// The view's content was already escaped for the positions it was built
    /// for, so this writer's context does not apply. The view's size hint is
    /// added to this writer's, so a view appended twice counts twice.
    ///
    /// # Panics
    ///
    /// Panics if the handle is nested and belongs to a different build.
    #[inline]
    pub fn push_view_handle(&mut self, handle: ViewHandle) -> &mut Self {
        self.size_hint += handle.size_hint();
        self.sink.push_view(handle);
        self
    }

    /// Records a response status code. Renders no content.
    #[cfg(feature = "http")]
    #[inline]
    pub fn push_status_code(&mut self, status_code: StatusCode) -> &mut Self {
        self.sink.push_status_code(status_code);
        self
    }

    /// Records response headers. Renders no content.
    #[cfg(feature = "http")]
    #[inline]
    pub fn push_headers(&mut self, headers: HeaderMap) -> &mut Self {
        self.sink.push_headers(headers);
        self
    }
}

macro_rules! impl_sink_primitive {
    ($method:ident, $ty:ty, $part:expr) => {
        #[inline]
        fn $method(&mut self, value: $ty) {
            match self {
                Self::Buffer(buffer) => buffer.$method(value),
                Self::Collector { collector, cx } => collector.push(cx, $part(value)),
            }
        }
    };
}

/// Where a writer's pushes go.
enum Sink<'a> {
    /// A view buffer under construction.
    Buffer(&'a mut ViewBuffer),
    /// The collector capturing one attribute key or value, with the request
    /// context used to render parts that cannot be held as is.
    Collector {
        collector: &'a mut AttributeCollector,
        cx: &'a Cx,
    },
}

impl Sink<'_> {
    #[inline]
    fn push_str(&mut self, value: &str, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_str(value, context),
            Self::Collector { collector, cx } => collector.push_str(cx, value, context),
        }
    }

    #[inline]
    fn push_static_str(&mut self, value: &'static str, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_static_str(value, context),
            Self::Collector { collector, cx } => {
                collector.push(cx, CollectedPart::StaticStr { value, context });
            }
        }
    }

    #[inline]
    fn push_promoted_str(&mut self, value: &'static &'static str, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_promoted_str(value, context),
            Self::Collector { collector, cx } => {
                collector.push(cx, CollectedPart::PromotedStr { value, context });
            }
        }
    }

    #[inline]
    fn push_string(&mut self, value: String, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_string(value, context),
            Self::Collector { collector, cx } => {
                collector.push(cx, CollectedPart::String { value, context });
            }
        }
    }

    #[inline]
    fn push_char(&mut self, value: char, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_char(value, context),
            Self::Collector { collector, cx } => {
                collector.push(cx, CollectedPart::Char { value, context });
            }
        }
    }

    impl_sink_primitive!(push_bool, bool, CollectedPart::Bool);
    impl_sink_primitive!(push_i8, i8, |value| CollectedPart::Int(i128::from(value)));
    impl_sink_primitive!(push_i16, i16, |value| CollectedPart::Int(i128::from(value)));
    impl_sink_primitive!(push_i32, i32, |value| CollectedPart::Int(i128::from(value)));
    impl_sink_primitive!(push_i64, i64, |value| CollectedPart::Int(i128::from(value)));
    impl_sink_primitive!(push_i128, i128, CollectedPart::Int);
    impl_sink_primitive!(push_isize, isize, |value| CollectedPart::Int(value as i128));
    impl_sink_primitive!(push_u8, u8, |value| CollectedPart::Uint(u128::from(value)));
    impl_sink_primitive!(push_u16, u16, |value| CollectedPart::Uint(u128::from(
        value
    )));
    impl_sink_primitive!(push_u32, u32, |value| CollectedPart::Uint(u128::from(
        value
    )));
    impl_sink_primitive!(push_u64, u64, |value| CollectedPart::Uint(u128::from(
        value
    )));
    impl_sink_primitive!(push_u128, u128, CollectedPart::Uint);
    impl_sink_primitive!(push_usize, usize, |value| CollectedPart::Uint(
        value as u128
    ));
    impl_sink_primitive!(push_f32, f32, CollectedPart::F32);
    impl_sink_primitive!(push_f64, f64, CollectedPart::F64);

    #[inline]
    fn push_region_start(&mut self, region: RegionId) {
        match self {
            Self::Buffer(buffer) => buffer.push_region_start(region),
            Self::Collector { .. } => panic!("tried to push a region into an attribute"),
        }
    }

    #[inline]
    fn push_region_end(&mut self, region: RegionId) {
        match self {
            Self::Buffer(buffer) => buffer.push_region_end(region),
            Self::Collector { .. } => panic!("tried to push a region into an attribute"),
        }
    }

    #[inline]
    fn push_dyn(&mut self, part: Box<dyn DynViewPart>, context: HtmlContext) {
        match self {
            Self::Buffer(buffer) => buffer.push_dyn(part, context),
            Self::Collector { collector, cx } => {
                collector.push(cx, CollectedPart::Dyn { part, context });
            }
        }
    }

    #[inline]
    fn push_view(&mut self, handle: ViewHandle) {
        match self {
            Self::Buffer(buffer) => buffer.push_view(handle),
            Self::Collector { collector, cx } => collector.push(cx, CollectedPart::View(handle)),
        }
    }

    /// Records a status code; a collected attribute has nowhere to keep one.
    #[cfg(feature = "http")]
    #[inline]
    fn push_status_code(&mut self, status_code: StatusCode) {
        if let Self::Buffer(buffer) = self {
            buffer.push_status_code(status_code);
        }
    }

    /// Records headers; a collected attribute has nowhere to keep them.
    #[cfg(feature = "http")]
    #[inline]
    fn push_headers(&mut self, headers: HeaderMap) {
        if let Self::Buffer(buffer) = self {
            buffer.push_headers(headers);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a view through a writer sealed with `context` and renders it.
    fn render_with(context: HtmlContext, f: impl FnOnce(&mut PartsWriter<'_>)) -> String {
        ViewBuffer::build(|parts| parts.in_context(context, f)).render(&Cx::default())
    }

    #[test]
    fn push_str_seals_the_writer_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_str("<b> & \"q\"");
        });
        assert_eq!(out, "&lt;b&gt; &amp; \"q\"");

        let out = render_with(HtmlContext::AttributeValue, |w| {
            w.push_str("<b> & \"q\"");
        });
        assert_eq!(out, "<b> &amp; &quot;q&quot;");
    }

    #[test]
    fn push_str_unescaped_bypasses_the_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_str_unescaped("<b>raw</b>");
        });
        assert_eq!(out, "<b>raw</b>");
    }

    #[test]
    fn push_promoted_str_seals_the_writer_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_promoted_str(&"<b> & \"q\"");
        });
        assert_eq!(out, "&lt;b&gt; &amp; \"q\"");

        let out = render_with(HtmlContext::AttributeValue, |w| {
            w.push_promoted_str(&"<b> & \"q\"");
        });
        assert_eq!(out, "<b> &amp; &quot;q&quot;");
    }

    #[test]
    fn push_promoted_str_unescaped_bypasses_the_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_promoted_str_unescaped(&"<b>raw</b>");
        });
        assert_eq!(out, "<b>raw</b>");
    }

    #[test]
    fn push_promoted_str_skips_empty_strings() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_promoted_str(&"a").push_promoted_str(&"");
            w.push_promoted_str_unescaped(&"").push_promoted_str(&"b");
        });
        assert_eq!(out, "ab");
    }

    #[test]
    fn push_char_seals_the_writer_context() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_char('<');
        });
        assert_eq!(out, "&lt;");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn ident_context_panics_on_forbidden_characters_at_render() {
        render_with(HtmlContext::AttributeKey, |w| {
            w.push_str("on click");
        });
    }

    #[test]
    fn push_primitives_render_as_text() {
        let out = render_with(HtmlContext::Text, |w| {
            w.push_i32(-42).push_str_unescaped(" ");
            w.push_bool(true).push_str_unescaped(" ");
            w.push_f64(1.5).push_str_unescaped(" ");
            w.push_i128(-1 << 100).push_str_unescaped(" ");
            w.push_u128(1 << 100);
        });
        assert_eq!(
            out,
            "-42 true 1.5 -1267650600228229401496703205376 1267650600228229401496703205376"
        );
    }
}
