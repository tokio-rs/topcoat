use core::fmt;
use std::borrow::Cow;

#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};
use smallvec::SmallVec;
use topcoat_core::context::Cx;

use crate::{Formatter, HtmlContext, HtmlWriter};

/// A self-contained piece of HTML content.
///
/// A view may contain multiple sibling nodes, but opened tags must be closed
/// so the fragment can be nested safely inside a larger document.
///
/// ```html
/// <!-- Valid: all tags are closed, safe to nest -->
/// <div>Hello</div>
/// <p>World</p>
///
/// <!-- Invalid: unclosed tag would corrupt the parent document -->
/// <div>Hello
/// ```
#[derive(Debug, Default, Clone)]
pub struct View {
    part: ViewPart,
}

impl View {
    /// Creates a view from accumulated view parts.
    ///
    /// This is called by generated `view!` code after collecting the nodes
    /// and attributes for a fragment.
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub fn new(parts: ViewParts) -> Self {
        Self { part: parts.into() }
    }

    /// Returns a `View` that renders to an empty string.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a view from a `&'static str` without escaping it and without checking for syntax
    /// errors.
    #[inline]
    #[must_use]
    pub const fn unescaped_unchecked(body: &'static str) -> Self {
        Self {
            part: ViewPart::unescaped(body),
        }
    }

    /// Renders the view into an HTML string.
    #[cfg_attr(
        feature = "http",
        doc = "",
        doc = "Status codes and headers declared in the view are discarded;",
        doc = "[`render_response`](Self::render_response) collects them."
    )]
    pub fn render(&self, cx: &Cx) -> String {
        let mut buf = String::with_capacity(self.part.size_hint());
        let mut f = Formatter::new(&mut buf);
        self.part.render(cx, &mut f);
        buf
    }

    /// Renders the view into HTML together with the status code and response
    /// headers declared in it.
    ///
    /// A view declares response metadata by placing an
    /// [`http::StatusCode`](StatusCode), an [`http::HeaderMap`](HeaderMap),
    /// or a single `(HeaderName, HeaderValue)` pair in the node position of
    /// the `view!` macro. Competing declarations resolve by render order:
    /// the first status code rendered wins, and the first part that mentions
    /// a header name provides all of that name's values.
    #[cfg(feature = "http")]
    #[must_use]
    pub fn render_response(&self, cx: &Cx) -> RenderedResponse {
        let mut html = String::with_capacity(self.part.size_hint());
        let mut f = Formatter::new(&mut html);
        self.part.render(cx, &mut f);
        let (status_code, headers) = f.into_recorded();
        RenderedResponse {
            html,
            status_code,
            headers,
        }
    }

    /// Unwraps the view into its root part.
    #[inline]
    pub(crate) fn into_part(self) -> ViewPart {
        self.part
    }
}

/// The output of rendering a [`View`] for an HTTP response.
///
/// Returned by [`View::render_response`]: the rendered HTML alongside the
/// status code and headers the view declared.
#[cfg(feature = "http")]
#[derive(Debug)]
#[non_exhaustive]
pub struct RenderedResponse {
    /// The rendered HTML.
    pub html: String,
    /// The first status code the render encountered, if any.
    pub status_code: Option<StatusCode>,
    /// The collected response headers.
    ///
    /// Each name carries the values of the first render part that mentioned
    /// it.
    pub headers: HeaderMap,
}

/// A renderable value stored in a [`View`].
///
/// View parts are created through a [`PartsWriter`] or the `view!` macro. A
/// part that holds text also records the [`HtmlContext`] it was written for,
/// so rendering escapes or validates it for exactly that position.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub enum ViewPart {
    /// Renders no content.
    #[default]
    Empty,
    /// A boolean rendered as text.
    #[non_exhaustive]
    Bool(bool),
    /// An `i8` rendered as text.
    #[non_exhaustive]
    I8(i8),
    /// An `i16` rendered as text.
    #[non_exhaustive]
    I16(i16),
    /// An `i32` rendered as text.
    #[non_exhaustive]
    I32(i32),
    /// An `i64` rendered as text.
    #[non_exhaustive]
    I64(i64),
    /// An `i128` rendered as text.
    #[non_exhaustive]
    I128(i128),
    /// An `isize` rendered as text.
    #[non_exhaustive]
    Isize(isize),
    /// A `u8` rendered as text.
    #[non_exhaustive]
    U8(u8),
    /// A `u16` rendered as text.
    #[non_exhaustive]
    U16(u16),
    /// A `u32` rendered as text.
    #[non_exhaustive]
    U32(u32),
    /// A `u64` rendered as text.
    #[non_exhaustive]
    U64(u64),
    /// A `u128` rendered as text.
    #[non_exhaustive]
    U128(u128),
    /// A `usize` rendered as text.
    #[non_exhaustive]
    Usize(usize),
    /// An `f32` rendered as text.
    #[non_exhaustive]
    F32(f32),
    /// An `f64` rendered as text.
    #[non_exhaustive]
    F64(f64),
    /// A character rendered for the recorded context.
    #[non_exhaustive]
    Char { value: char, context: HtmlContext },
    /// A string rendered for the recorded context.
    #[non_exhaustive]
    Str {
        value: Cow<'static, str>,
        context: HtmlContext,
    },
    /// A custom view part that writes its output at render time.
    #[non_exhaustive]
    BoxDyn {
        inner: Box<dyn DynViewPart>,
        context: HtmlContext,
        size_hint: usize,
    },
    /// A sequence of view parts rendered in order.
    #[non_exhaustive]
    BoxSlice {
        inner: Box<[ViewPart]>,
        size_hint: usize,
    },
    /// A response status code recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    StatusCode(StatusCode),
    /// Response headers recorded at render time; renders no content.
    #[cfg(feature = "http")]
    #[non_exhaustive]
    Headers(Box<HeaderMap>),
}

impl ViewPart {
    /// Returns an empty view part.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::Empty
    }

    /// Returns `true` if the view part is [`Empty`].
    ///
    /// [`Empty`]: ViewPart::Empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns a part that renders `value` verbatim.
    #[inline]
    pub(crate) const fn unescaped(value: &'static str) -> Self {
        Self::Str {
            value: Cow::Borrowed(value),
            context: HtmlContext::Unescaped,
        }
    }

    /// Writes the part into `f`, escaped or validated for the context each
    /// piece of text was written in.
    pub(crate) fn render(&self, cx: &Cx, f: &mut Formatter<'_>) {
        let mut int_buffer = itoa::Buffer::new();
        let mut float_buffer = zmij::Buffer::new();

        match self {
            Self::Empty => {}
            Self::Bool(inner) => f.write_str(if *inner { "true" } else { "false" }),
            // The `Display` output of the numeric types consists of digits,
            // signs, and plain letters, none of which are significant in any
            // HTML context, so they write verbatim.
            Self::I8(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I16(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I32(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I64(inner) => f.write_str(int_buffer.format(*inner)),
            Self::I128(inner) => f.write_str(int_buffer.format(*inner)),
            Self::Isize(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U8(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U16(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U32(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U64(inner) => f.write_str(int_buffer.format(*inner)),
            Self::U128(inner) => f.write_str(int_buffer.format(*inner)),
            Self::Usize(inner) => f.write_str(int_buffer.format(*inner)),
            Self::F32(inner) => f.write_str(float_buffer.format(*inner)),
            Self::F64(inner) => f.write_str(float_buffer.format(*inner)),
            Self::Char { value, context } => context.writer(f).write_char(*value),
            Self::Str { value, context } => context.writer(f).write_str(value),
            Self::BoxDyn { inner, context, .. } => inner.render(cx, &mut context.writer(f)),
            Self::BoxSlice { inner, .. } => {
                for part in inner {
                    part.render(cx, f);
                }
            }
            #[cfg(feature = "http")]
            Self::StatusCode(status_code) => f.record_status_code(*status_code),
            #[cfg(feature = "http")]
            Self::Headers(headers) => f.record_headers(headers),
        }
    }

    /// Returns an estimate of the number of bytes this part will write.
    ///
    /// Used to pre-allocate the output buffer. A slight over-estimate is
    /// preferable to an under-estimate: falling short forces the buffer to
    /// grow and copy, whereas a modest over-estimate only leaves a little
    /// capacity unused.
    pub(crate) fn size_hint(&self) -> usize {
        // Each numeric hint is the midpoint, rounded up, between the shortest
        // and widest output the type can `Display`, including the leading `-`
        // for signed types (`isize`/`usize` assume a 64-bit target). A
        // float's `Display` width is unbounded for extreme magnitudes, so the
        // upper end is the shortest round-trip form of a typical value.
        #[allow(clippy::match_same_arms)]
        match self {
            Self::Empty => 0,
            Self::Bool(_) => 5,
            Self::I8(_) => 3,
            Self::I16(_) => 4,
            Self::I32(_) => 6,
            Self::I64(_) => 11,
            Self::I128(_) => 21,
            Self::Isize(_) => 11,
            Self::U8(_) => 2,
            Self::U16(_) => 3,
            Self::U32(_) => 6,
            Self::U64(_) => 11,
            Self::U128(_) => 20,
            Self::Usize(_) => 11,
            Self::F32(_) => 9,
            Self::F64(_) => 13,
            // One to four UTF-8 bytes, or an escape sequence.
            Self::Char { .. } => 3,
            Self::Str { value, context } => match context {
                HtmlContext::Unescaped => value.len(),
                // Assume some characters escape into multi-byte sequences.
                _ => value.len() + value.len() / 8,
            },
            Self::BoxDyn { size_hint, .. } | Self::BoxSlice { size_hint, .. } => *size_hint,
            #[cfg(feature = "http")]
            Self::StatusCode(_) | Self::Headers(_) => 0,
        }
    }
}

/// A boxed view part that writes its output at render time.
///
/// Implement this for values whose output is only known when the view
/// renders, such as resolved asset URLs. The writer passed to
/// [`render`](Self::render) already carries the [`HtmlContext`] of the
/// position the part was pushed into, so everything written through it is
/// escaped or validated for that position.
pub trait DynViewPart: 'static + fmt::Debug + Send {
    /// Writes this part's output into `w`.
    fn render(&self, cx: &Cx, w: &mut HtmlWriter<'_, '_>);

    /// Returns an estimate of the number of bytes this part will write.
    ///
    /// Used to pre-allocate the output buffer, so aim for a close estimate. A
    /// slight over-estimate is usually preferable to an under-estimate.
    #[inline]
    fn size_hint(&self) -> usize {
        0
    }

    /// Clones this view part into a fresh boxed value.
    fn clone_box(&self) -> Box<dyn DynViewPart>;
}

impl Clone for Box<dyn DynViewPart> {
    #[inline]
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

/// A buffer collecting renderable values before they become a [`View`].
///
/// This is plumbing for generated `view!` code, which fills the buffer
/// through [`PartsWriter`] lenses and the position helpers and finally passes
/// it to `View::new`.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct ViewParts {
    items: SmallVec<[ViewPart; 8]>,
}

impl ViewParts {
    /// Creates an empty view-parts buffer.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a nested view, such as a rendered component.
    #[inline]
    pub fn push_view(&mut self, view: View) -> &mut Self {
        self.items.push(view.into_part());
        self
    }

    /// Appends an already-sealed view part.
    ///
    /// A part records the [`HtmlContext`] its text was written for. Pushing
    /// it into a position with different escaping requirements bypasses that
    /// protection, so this is reserved for framework plumbing that re-emits
    /// parts in the position family they were built for.
    #[doc(hidden)]
    #[inline]
    pub fn push_part(&mut self, part: ViewPart) -> &mut Self {
        self.items.push(part);
        self
    }
}

impl From<ViewParts> for ViewPart {
    #[inline]
    fn from(mut value: ViewParts) -> Self {
        match value.items.len() {
            0 => ViewPart::Empty,
            1 => value.items.pop().unwrap(),
            _ => {
                let size_hint = value.items.iter().map(ViewPart::size_hint).sum();
                ViewPart::BoxSlice {
                    inner: value.items.into_boxed_slice(),
                    size_hint,
                }
            }
        }
    }
}

macro_rules! impl_push_primitive {
    ($method:ident, $ty:ty, $variant:ident) => {
        #[doc = concat!("Appends a `", stringify!($ty), "` rendered as text.")]
        ///
        /// Its rendered form contains no character that is significant in any
        /// HTML context, so no escaping applies.
        #[inline]
        pub fn $method(&mut self, value: $ty) -> &mut Self {
            self.parts.items.push(ViewPart::$variant(value));
            self
        }
    };
}

/// A context-carrying writer over a view-parts buffer, created per position.
///
/// The `view!` macro creates a `PartsWriter` for each dynamic position it
/// fills and hands it to the matching position trait:
/// [`NodeViewParts`](crate::NodeViewParts),
/// [`AttributeValueViewParts`](crate::AttributeValueViewParts),
/// [`AttributeKeyViewParts`](crate::AttributeKeyViewParts),
/// [`ElementNameViewParts`](crate::ElementNameViewParts), or
/// [`AttributeViewParts`](crate::AttributeViewParts).
///
/// Implementations of those traits make a value renderable by pushing it
/// through the `push_*` methods, which seal the pushed text with the
/// [`HtmlContext`] of the position so rendering escapes or validates it
/// correctly, or by delegating to another implementation of the same
/// position trait. [`push_str_unescaped`](Self::push_str_unescaped) is the
/// only way to opt out of that protection.
pub struct PartsWriter<'a> {
    parts: &'a mut ViewParts,
    context: HtmlContext,
}

impl<'a> PartsWriter<'a> {
    /// Creates a writer that seals everything pushed into it with `context`.
    #[inline]
    pub fn new(parts: &'a mut ViewParts, context: HtmlContext) -> Self {
        Self { parts, context }
    }

    /// Returns a writer over the same buffer for a different context.
    ///
    /// In-crate compositions that span more than one position use this to
    /// transition between the positions they cover, such as
    /// [`Attribute`](crate::Attribute) moving from a key to a value or
    /// [`push_comment`](Self::push_comment) sealing a comment body.
    #[inline]
    pub(crate) fn with_context(&mut self, context: HtmlContext) -> PartsWriter<'_> {
        PartsWriter {
            parts: self.parts,
            context,
        }
    }

    /// Appends a string, sealed with this writer's context.
    #[inline]
    pub fn push_str(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.parts.items.push(ViewPart::Str {
            value: value.into(),
            context: self.context,
        });
        self
    }

    /// Appends a string that renders verbatim, bypassing this writer's
    /// context.
    ///
    /// Use this only for trusted markup. Passing untrusted input defeats the
    /// runtime's escaping and can lead to XSS vulnerabilities.
    #[inline]
    pub fn push_str_unescaped(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.parts.items.push(ViewPart::Str {
            value: value.into(),
            context: HtmlContext::Unescaped,
        });
        self
    }

    /// Appends an HTML comment whose body is built through `build`.
    ///
    /// The `<!-- ` and ` -->` delimiters are written verbatim, while the
    /// writer handed to `build` seals everything pushed into it for the
    /// [`Comment`](HtmlContext::Comment) context. Because that context
    /// escapes `>`, the body can never contain `-->` and terminate the
    /// comment, so a marker can be built from untrusted data with
    /// [`push_str`](Self::push_str) and no separate escaping step.
    ///
    /// # Panics
    ///
    /// Panics if used in a non-text HTML context.
    #[inline]
    pub fn push_comment(&mut self, build: impl FnOnce(&mut PartsWriter<'_>)) -> &mut Self {
        assert!(
            self.context == HtmlContext::Text,
            "tried to push comment in html context {:?}",
            self.context,
        );
        self.push_str_unescaped("<!-- ");
        build(&mut self.with_context(HtmlContext::Comment));
        self.push_str_unescaped(" -->");
        self
    }

    /// Appends a character, sealed with this writer's context.
    #[inline]
    pub fn push_char(&mut self, value: char) -> &mut Self {
        self.parts.items.push(ViewPart::Char {
            value,
            context: self.context,
        });
        self
    }

    impl_push_primitive!(push_bool, bool, Bool);
    impl_push_primitive!(push_i8, i8, I8);
    impl_push_primitive!(push_i16, i16, I16);
    impl_push_primitive!(push_i32, i32, I32);
    impl_push_primitive!(push_i64, i64, I64);
    impl_push_primitive!(push_i128, i128, I128);
    impl_push_primitive!(push_isize, isize, Isize);
    impl_push_primitive!(push_u8, u8, U8);
    impl_push_primitive!(push_u16, u16, U16);
    impl_push_primitive!(push_u32, u32, U32);
    impl_push_primitive!(push_u64, u64, U64);
    impl_push_primitive!(push_u128, u128, U128);
    impl_push_primitive!(push_usize, usize, Usize);
    impl_push_primitive!(push_f32, f32, F32);
    impl_push_primitive!(push_f64, f64, F64);

    /// Appends a part that writes its output at render time, sealed with
    /// this writer's context.
    #[inline]
    pub fn push_dyn(&mut self, part: Box<dyn DynViewPart>) -> &mut Self {
        self.parts.items.push(ViewPart::BoxDyn {
            size_hint: part.size_hint(),
            inner: part,
            context: self.context,
        });
        self
    }

    /// Appends an already-sealed view part.
    ///
    /// A part records the [`HtmlContext`] its text was written for; this
    /// writer's context does not apply. See [`ViewParts::push_part`].
    #[doc(hidden)]
    #[inline]
    pub fn push_part(&mut self, part: ViewPart) -> &mut Self {
        self.parts.items.push(part);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(build: impl FnOnce(&mut ViewParts)) -> String {
        let mut parts = ViewParts::new();
        build(&mut parts);
        View::new(parts).render(&Cx::default())
    }

    #[test]
    fn empty_view_renders_empty() {
        assert_eq!(View::empty().render(&Cx::default()), "");
    }

    #[test]
    fn unescaped_unchecked_renders_verbatim() {
        let view = View::unescaped_unchecked("<b>raw</b>");
        assert_eq!(view.render(&Cx::default()), "<b>raw</b>");
    }

    #[test]
    fn push_str_seals_the_writer_context() {
        let out = render(|parts| {
            PartsWriter::new(parts, HtmlContext::Text).push_str("<b> & \"q\"");
        });
        assert_eq!(out, "&lt;b&gt; &amp; \"q\"");

        let out = render(|parts| {
            PartsWriter::new(parts, HtmlContext::AttributeValue).push_str("<b> & \"q\"");
        });
        assert_eq!(out, "<b> &amp; &quot;q&quot;");
    }

    #[test]
    fn push_str_unescaped_bypasses_the_context() {
        let out = render(|parts| {
            PartsWriter::new(parts, HtmlContext::Text).push_str_unescaped("<b>raw</b>");
        });
        assert_eq!(out, "<b>raw</b>");
    }

    #[test]
    fn push_char_seals_the_writer_context() {
        let out = render(|parts| {
            PartsWriter::new(parts, HtmlContext::Text).push_char('<');
        });
        assert_eq!(out, "&lt;");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn ident_context_panics_on_forbidden_characters_at_render() {
        render(|parts| {
            PartsWriter::new(parts, HtmlContext::AttributeKey).push_str("on click");
        });
    }

    #[test]
    fn push_primitives_render_as_text() {
        let out = render(|parts| {
            let mut writer = PartsWriter::new(parts, HtmlContext::Text);
            writer.push_i32(-42).push_str_unescaped(" ");
            writer.push_bool(true).push_str_unescaped(" ");
            writer.push_f64(1.5);
        });
        assert_eq!(out, "-42 true 1.5");
    }

    #[test]
    fn push_view_splices_nested_views() {
        let mut inner_parts = ViewParts::new();
        PartsWriter::new(&mut inner_parts, HtmlContext::Text).push_str("a < b");
        let inner = View::new(inner_parts);

        let out = render(|parts| {
            PartsWriter::new(parts, HtmlContext::Unescaped).push_str("<p>");
            parts.push_view(inner);
            PartsWriter::new(parts, HtmlContext::Unescaped).push_str("</p>");
        });
        assert_eq!(out, "<p>a &lt; b</p>");
    }

    #[test]
    fn size_hint_is_exact_for_unescaped_strings() {
        let view = View::unescaped_unchecked("<b>raw</b>");
        assert_eq!(view.part.size_hint(), 10);
    }

    #[cfg(feature = "http")]
    mod response {
        use http::header::{CACHE_CONTROL, SET_COOKIE};
        use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

        use super::*;
        use crate::NodeViewParts;

        fn push_node(parts: &mut ViewParts, value: impl NodeViewParts) {
            value.into_view_parts(
                &Cx::default(),
                &mut PartsWriter::new(parts, HtmlContext::Text),
            );
        }

        fn push_text(parts: &mut ViewParts, text: &'static str) {
            PartsWriter::new(parts, HtmlContext::Text).push_str(text);
        }

        #[test]
        fn status_code_is_recorded_and_renders_nothing() {
            let mut parts = ViewParts::new();
            push_text(&mut parts, "a");
            push_node(&mut parts, StatusCode::NOT_FOUND);
            push_text(&mut parts, "b");

            let rendered = View::new(parts).render_response(&Cx::default());
            assert_eq!(rendered.html, "ab");
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
            assert!(rendered.headers.is_empty());
        }

        #[test]
        fn render_response_without_declarations_is_empty() {
            let mut parts = ViewParts::new();
            push_text(&mut parts, "a");

            let rendered = View::new(parts).render_response(&Cx::default());
            assert_eq!(rendered.html, "a");
            assert_eq!(rendered.status_code, None);
            assert!(rendered.headers.is_empty());
        }

        #[test]
        fn render_discards_declarations() {
            let mut parts = ViewParts::new();
            push_node(&mut parts, StatusCode::NOT_FOUND);
            push_node(
                &mut parts,
                (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            );
            push_text(&mut parts, "a");

            assert_eq!(View::new(parts).render(&Cx::default()), "a");
        }

        #[test]
        fn first_status_code_wins() {
            let mut parts = ViewParts::new();
            push_node(&mut parts, StatusCode::NOT_FOUND);
            push_node(&mut parts, StatusCode::OK);

            let rendered = View::new(parts).render_response(&Cx::default());
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
        }

        #[test]
        fn first_mention_of_a_header_name_wins() {
            let mut parts = ViewParts::new();
            push_node(
                &mut parts,
                (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            );
            let mut later = HeaderMap::new();
            later.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));
            later.insert(
                HeaderName::from_static("x-extra"),
                HeaderValue::from_static("1"),
            );
            push_node(&mut parts, later);

            let rendered = View::new(parts).render_response(&Cx::default());
            assert_eq!(rendered.headers[CACHE_CONTROL], "no-store");
            assert_eq!(rendered.headers["x-extra"], "1");
        }

        #[test]
        fn one_map_keeps_all_values_for_a_name() {
            let mut first = HeaderMap::new();
            first.append(SET_COOKIE, HeaderValue::from_static("a=1"));
            first.append(SET_COOKIE, HeaderValue::from_static("b=2"));
            let mut later = HeaderMap::new();
            later.insert(SET_COOKIE, HeaderValue::from_static("c=3"));

            let mut parts = ViewParts::new();
            push_node(&mut parts, first);
            push_node(&mut parts, later);

            let rendered = View::new(parts).render_response(&Cx::default());
            let cookies: Vec<_> = rendered.headers.get_all(SET_COOKIE).iter().collect();
            assert_eq!(cookies, ["a=1", "b=2"]);
        }

        #[test]
        fn placement_decides_precedence_across_nested_views() {
            let mut inner_parts = ViewParts::new();
            push_node(&mut inner_parts, StatusCode::NOT_FOUND);
            push_text(&mut inner_parts, "inner");
            let inner = View::new(inner_parts);

            // A status code before the nested view overrides it.
            let mut outer_parts = ViewParts::new();
            push_node(&mut outer_parts, StatusCode::FORBIDDEN);
            outer_parts.push_view(inner.clone());
            let rendered = View::new(outer_parts).render_response(&Cx::default());
            assert_eq!(rendered.status_code, Some(StatusCode::FORBIDDEN));

            // A status code after the nested view is only a fallback.
            let mut outer_parts = ViewParts::new();
            outer_parts.push_view(inner);
            push_node(&mut outer_parts, StatusCode::FORBIDDEN);
            let rendered = View::new(outer_parts).render_response(&Cx::default());
            assert_eq!(rendered.status_code, Some(StatusCode::NOT_FOUND));
        }
    }
}
