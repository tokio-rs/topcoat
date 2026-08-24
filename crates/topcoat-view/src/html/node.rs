use std::{borrow::Cow, pin::pin};

use futures_util::StreamExt;
#[cfg(feature = "http")]
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    BoxView, Child, PartsWriter, PromotedStr, StaticStr, Unescaped, ViewChunk, ViewHandle,
    ViewStream,
    buffer::ViewBufferScope,
    yielder::yield_,
};

/// Converts a value used in node position into view parts.
///
/// When this trait is implemented on a type, it can be used in the node position of an element
/// in the [`view!`](https://docs.rs/topcoat/latest/topcoat/view/macro.view.html) macro:
///
/// ```rust
/// # use topcoat::view::{component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// # let my_value = "value";
/// view! {
///     <div>(my_value)</div>
/// }
/// # }
/// ```
pub trait NodeViewParts {
    /// Appends this value to the view being built.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

/// Streams a value used in node position as [`ViewChunk`]s.
///
/// The `view!` macro drives every dynamic node position through this trait.
/// Plain values get it through the blanket impl over [`NodeViewParts`],
/// emitting their parts as a single chunk; view streams implement it
/// directly, forwarding a chunk at a time.
pub trait NodeViewPartsStream {
    /// Whether this value may emit more than one chunk at its position.
    ///
    /// The template surrounds a multi-chunk position with marker comments,
    /// so a later chunk can be swapped in on the client.
    const MULTI: bool;

    /// Emits this value's chunks through `writer`.
    fn into_view_parts_stream<'cx>(
        self,
        cx: &'cx Cx,
        writer: NodeWriter,
    ) -> impl Future<Output = Result<()>> + Send + 'cx
    where
        Self: 'cx;
}

impl<T: NodeViewParts + Send> NodeViewPartsStream for T {
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(self, cx: &'cx Cx, mut writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        writer.emit(|parts| self.into_view_parts(cx, parts)).await;
        Ok(())
    }
}

/// A view interpolated into a node position re-emits each of its chunks at
/// the position; an error its stream yields fails the position.
impl<F> NodeViewPartsStream for ViewStream<F>
where
    F: Future<Output = Result<()>> + Send,
{
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(self, _cx: &'cx Cx, mut writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        let mut stream = pin!(self);
        while let Some(chunk) = stream.next().await {
            match chunk? {
                ViewChunk::Content(view) => {
                    writer.emit(|parts| {
                        parts.push_view(view);
                    })
                    .await;
                }
                // A swap targets its own position; it passes through as is.
                chunk @ ViewChunk::Swap { .. } => yield_(Ok(chunk)).await,
            }
        }
        Ok(())
    }
}

impl NodeViewPartsStream for BoxView<'_> {
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(mut self, _cx: &'cx Cx, mut writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        while let Some(chunk) = self.next().await {
            match chunk? {
                ViewChunk::Content(view) => {
                    writer.emit(|parts| {
                        parts.push_view(view);
                    })
                    .await;
                }
                // A swap targets its own position; it passes through as is.
                chunk @ ViewChunk::Swap { .. } => yield_(Ok(chunk)).await,
            }
        }
        Ok(())
    }
}

impl NodeViewPartsStream for Child<'_> {
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(self, cx: &'cx Cx, writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        self.view.into_view_parts_stream(cx, writer).await
    }
}

/// The emission handle of a node position: each [`emit`](Self::emit) call
/// yields one [`ViewChunk`] filled by the caller.
///
/// Handed to [`NodeViewPartsStream::into_view_parts_stream`] by the `view!`
/// machinery driving the position. Most implementations emit exactly once;
/// only a position declaring [`MULTI`](NodeViewPartsStream::MULTI) may emit
/// again to replace its content.
pub struct NodeWriter {
    _private: (),
}

impl NodeWriter {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    /// Builds one chunk's view in a synchronous burst through the writer
    /// handed to `f`, and yields it through the enclosing stream.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub async fn emit(&mut self, f: impl FnOnce(&mut PartsWriter<'_>)) {
        let view = ViewBufferScope::with(|buffer| PartsWriter::block(buffer, f));
        yield_(Ok(ViewChunk::Content(view))).await;
    }
}

impl NodeViewParts for ViewHandle {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_view(self);
    }
}

macro_rules! impl_primitive {
    ($ty:ty, $method:ident) => {
        impl NodeViewParts for $ty {
            #[inline]
            fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
                parts.$method(self);
            }
        }
    };
    ($ty:ty, $method:ident, ref) => {
        impl_primitive!($ty, $method);

        impl NodeViewParts for &$ty {
            #[inline]
            fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
                (*self).into_view_parts(cx, parts)
            }
        }
    };
}

impl_primitive!(bool, push_bool, ref);
impl_primitive!(char, push_char, ref);
impl_primitive!(i8, push_i8, ref);
impl_primitive!(i16, push_i16, ref);
impl_primitive!(i32, push_i32, ref);
impl_primitive!(i64, push_i64, ref);
impl_primitive!(i128, push_i128, ref);
impl_primitive!(isize, push_isize, ref);
impl_primitive!(u8, push_u8, ref);
impl_primitive!(u16, push_u16, ref);
impl_primitive!(u32, push_u32, ref);
impl_primitive!(u64, push_u64, ref);
impl_primitive!(u128, push_u128, ref);
impl_primitive!(usize, push_usize, ref);
impl_primitive!(f32, push_f32, ref);
impl_primitive!(f64, push_f64, ref);
impl_primitive!(String, push_string);

impl NodeViewParts for Cow<'static, str> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Cow::Borrowed(value) => parts.push_static_str(value),
            Cow::Owned(value) => parts.push_string(value),
        };
    }
}

impl NodeViewParts for &str {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self);
    }
}

impl NodeViewParts for PromotedStr {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str(self.0);
    }
}

impl NodeViewParts for StaticStr {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str(self.0);
    }
}

impl NodeViewParts for Unescaped<String> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_string_unescaped(self.0);
    }
}

impl NodeViewParts for Unescaped<PromotedStr> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_promoted_str_unescaped(self.0.0);
    }
}

impl NodeViewParts for Unescaped<StaticStr> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0.0);
    }
}

impl NodeViewParts for Unescaped<&'static str> {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.0);
    }
}

impl NodeViewParts for &String {
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.as_str().into_view_parts(cx, parts);
    }
}

/// Sets the response status code; renders no content.
///
/// Competing status codes resolve by render order: the first one rendered
/// wins. Place a status code before nested content to override whatever the
/// content declares, or after it to provide a fallback. To display a status
/// code as text instead, render one of its accessors, such as
/// [`as_u16`](StatusCode::as_u16).
#[cfg(feature = "http")]
impl NodeViewParts for StatusCode {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_status_code(self);
    }
}

/// Adds response headers; renders no content.
///
/// Competing headers resolve by render order: the first part that mentions a
/// header name provides all of that name's values. Place headers before
/// nested content to override the entries the content declares, or after it
/// to provide fallbacks.
#[cfg(feature = "http")]
impl NodeViewParts for HeaderMap {
    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_headers(self);
    }
}

/// Adds a single response header; renders no content.
///
/// Equivalent to a [`HeaderMap`] holding just this entry.
#[cfg(feature = "http")]
impl NodeViewParts for (HeaderName, HeaderValue) {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let (name, value) = self;
        let mut headers = HeaderMap::with_capacity(1);
        headers.insert(name, value);
        headers.into_view_parts(cx, parts);
    }
}

impl<'b, T: ?Sized> NodeViewParts for &&'b T
where
    &'b T: NodeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        (*self).into_view_parts(cx, parts);
    }
}

impl<T> NodeViewParts for Option<T>
where
    T: NodeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(value) = self {
            value.into_view_parts(cx, parts);
        }
    }
}

impl<T> NodeViewParts for Vec<T>
where
    T: NodeViewParts,
{
    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        for value in self {
            value.into_view_parts(cx, parts);
        }
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> NodeViewParts for ($($ty,)+)
        where
            $($ty: NodeViewParts,)+
        {
            #[inline]
            #[allow(non_snake_case)]
            fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
                let ($($ty,)+) = self;
                $($ty.into_view_parts(cx, parts);)+
            }
        }
    };
}

impl_tuple!(T1);
impl_tuple!(T1, T2);
impl_tuple!(T1, T2, T3);
impl_tuple!(T1, T2, T3, T4);
impl_tuple!(T1, T2, T3, T4, T5);
impl_tuple!(T1, T2, T3, T4, T5, T6);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
