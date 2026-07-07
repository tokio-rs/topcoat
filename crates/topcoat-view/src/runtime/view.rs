use core::fmt;

use topcoat_core::runtime::context::Cx;

use crate::runtime::{FmtHtml, Formatter, Unescaped};

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
    /// This is usually called by generated `view!` code after collecting the
    /// nodes and attributes for a fragment.
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
            part: ViewPart::UnescapedStaticStr(Unescaped::new_unchecked(body)),
        }
    }

    /// Renders the view into an HTML string.
    pub fn render(&self, cx: &Cx) -> String {
        let mut buf = String::with_capacity(self.size_hint());
        let mut f = Formatter::new(&mut buf);
        self.fmt_html(cx, &mut f);
        buf
    }
}

impl FmtHtml for View {
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        self.part.fmt_html(cx, f);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        self.part.size_hint()
    }
}

/// A renderable value stored in a [`View`].
///
/// Most code creates view parts through [`ViewParts::push`] or the `view!`
/// macro rather than constructing enum variants directly.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub enum ViewPart {
    /// Renders no content.
    #[default]
    Empty,
    /// A boolean rendered as text.
    #[non_exhaustive]
    Bool(bool),
    /// A character rendered as text.
    #[non_exhaustive]
    Char(char),
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
    /// A borrowed string rendered as escaped text.
    #[non_exhaustive]
    StaticStr(&'static str),
    /// An owned string rendered as escaped text.
    #[non_exhaustive]
    String(String),
    /// A borrowed string rendered without escaping.
    #[non_exhaustive]
    UnescapedStaticStr(Unescaped<&'static str>),
    /// An owned string rendered without escaping.
    #[non_exhaustive]
    UnescapedString(Unescaped<String>),
    /// A custom view part stored in a cloneable box.
    #[non_exhaustive]
    BoxDyn {
        inner: Box<dyn DynViewPart>,
        size_hint: usize,
    },
    /// A sequence of view parts rendered in order.
    #[non_exhaustive]
    BoxSlice {
        inner: Box<[ViewPart]>,
        size_hint: usize,
    },
    /// A Vec of view parts rendered in order.
    #[non_exhaustive]
    Vec {
        inner: Vec<ViewPart>,
        size_hint: usize,
    },
}

impl ViewPart {
    /// Returns an empty view part.
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        Self::Empty
    }
}

/// A boxed [`FmtHtml`] that can be cloned.
///
/// This is mainly useful when a custom HTML formattable type needs to
/// be stored in a [`ViewPart`].
pub trait DynViewPart: 'static + FmtHtml + fmt::Debug + Send {
    /// Clones this view part into a fresh boxed value.
    fn clone_box(&self) -> Box<dyn DynViewPart>;
}

impl<T> DynViewPart for T
where
    T: 'static + FmtHtml + fmt::Debug + Clone + Send,
{
    #[inline]
    fn clone_box(&self) -> Box<dyn DynViewPart> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn DynViewPart> {
    #[inline]
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

impl FmtHtml for ViewPart {
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        match self {
            Self::Empty => {}
            Self::Bool(inner) => inner.fmt_html(cx, f),
            Self::Char(inner) => inner.fmt_html(cx, f),
            Self::I8(inner) => inner.fmt_html(cx, f),
            Self::I16(inner) => inner.fmt_html(cx, f),
            Self::I32(inner) => inner.fmt_html(cx, f),
            Self::I64(inner) => inner.fmt_html(cx, f),
            Self::I128(inner) => inner.fmt_html(cx, f),
            Self::Isize(inner) => inner.fmt_html(cx, f),
            Self::U8(inner) => inner.fmt_html(cx, f),
            Self::U16(inner) => inner.fmt_html(cx, f),
            Self::U32(inner) => inner.fmt_html(cx, f),
            Self::U64(inner) => inner.fmt_html(cx, f),
            Self::U128(inner) => inner.fmt_html(cx, f),
            Self::Usize(inner) => inner.fmt_html(cx, f),
            Self::F32(inner) => inner.fmt_html(cx, f),
            Self::F64(inner) => inner.fmt_html(cx, f),
            Self::String(inner) => inner.fmt_html(cx, f),
            Self::StaticStr(inner) => inner.fmt_html(cx, f),
            Self::UnescapedString(inner) => inner.fmt_html(cx, f),
            Self::UnescapedStaticStr(inner) => inner.fmt_html(cx, f),
            Self::BoxDyn { inner, .. } => FmtHtml::fmt_html(inner, cx, f),
            Self::BoxSlice { inner, .. } => {
                for part in inner {
                    part.fmt_html(cx, f);
                }
            }
            Self::Vec { inner, .. } => {
                for part in inner {
                    part.fmt_html(cx, f);
                }
            }
        }
    }

    fn size_hint(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Bool(inner) => inner.size_hint(),
            Self::Char(inner) => inner.size_hint(),
            Self::I8(inner) => inner.size_hint(),
            Self::I16(inner) => inner.size_hint(),
            Self::I32(inner) => inner.size_hint(),
            Self::I64(inner) => inner.size_hint(),
            Self::I128(inner) => inner.size_hint(),
            Self::Isize(inner) => inner.size_hint(),
            Self::U8(inner) => inner.size_hint(),
            Self::U16(inner) => inner.size_hint(),
            Self::U32(inner) => inner.size_hint(),
            Self::U64(inner) => inner.size_hint(),
            Self::U128(inner) => inner.size_hint(),
            Self::Usize(inner) => inner.size_hint(),
            Self::F32(inner) => inner.size_hint(),
            Self::F64(inner) => inner.size_hint(),
            Self::StaticStr(inner) => inner.size_hint(),
            Self::String(inner) => inner.size_hint(),
            Self::UnescapedString(inner) => inner.len(),
            Self::UnescapedStaticStr(inner) => inner.len(),
            Self::BoxDyn { size_hint, .. }
            | Self::BoxSlice { size_hint, .. }
            | Self::Vec { size_hint, .. } => *size_hint,
        }
    }
}

macro_rules! impl_from_for_view_part {
    ($variant:ident($ty:ty)) => {
        impl From<$ty> for ViewPart {
            #[inline]
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_from_for_view_part! { Bool(bool) }
impl_from_for_view_part! { Char(char) }
impl_from_for_view_part! { I8(i8) }
impl_from_for_view_part! { I16(i16) }
impl_from_for_view_part! { I32(i32) }
impl_from_for_view_part! { I64(i64) }
impl_from_for_view_part! { I128(i128) }
impl_from_for_view_part! { Isize(isize) }
impl_from_for_view_part! { U8(u8) }
impl_from_for_view_part! { U16(u16) }
impl_from_for_view_part! { U32(u32) }
impl_from_for_view_part! { U64(u64) }
impl_from_for_view_part! { U128(u128) }
impl_from_for_view_part! { Usize(usize) }
impl_from_for_view_part! { F32(f32) }
impl_from_for_view_part! { F64(f64) }
impl_from_for_view_part! { StaticStr(&'static str) }
impl_from_for_view_part! { String(String) }
impl_from_for_view_part! { UnescapedStaticStr(Unescaped<&'static str>) }
impl_from_for_view_part! { UnescapedString(Unescaped<String>) }

impl From<Box<dyn DynViewPart>> for ViewPart {
    fn from(value: Box<dyn DynViewPart>) -> Self {
        Self::BoxDyn {
            size_hint: value.size_hint(),
            inner: value,
        }
    }
}

impl From<Box<[ViewPart]>> for ViewPart {
    fn from(value: Box<[ViewPart]>) -> Self {
        Self::BoxSlice {
            size_hint: value.iter().map(ViewPart::size_hint).sum(),
            inner: value,
        }
    }
}

impl From<Vec<ViewPart>> for ViewPart {
    fn from(value: Vec<ViewPart>) -> Self {
        Self::Vec {
            size_hint: value.iter().map(ViewPart::size_hint).sum(),
            inner: value,
        }
    }
}

impl From<View> for ViewPart {
    fn from(value: View) -> Self {
        value.part
    }
}

/// A builder for collecting renderable values before creating a [`View`].
///
/// Use [`push`](Self::push) to append values, then pass the builder to
/// [`View::new`].
#[derive(Debug, Default, Clone)]
pub struct ViewParts {
    first: Option<ViewPart>,
    items: Vec<ViewPart>,
}

impl ViewParts {
    /// Creates an empty view-parts builder.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a renderable part and returns the builder.
    #[inline]
    pub fn push(&mut self, part: impl Into<ViewPart>) -> &mut Self {
        let part = part.into();
        if let Some(first) = self.first.take() {
            self.items.push(first);
            self.items.push(part);
        } else if self.items.is_empty() {
            self.first = Some(part);
        } else {
            self.items.push(part);
        }
        self
    }
}

impl From<ViewParts> for ViewPart {
    #[inline]
    fn from(value: ViewParts) -> Self {
        if let Some(first) = value.first {
            first
        } else if value.items.is_empty() {
            ViewPart::empty()
        } else {
            value.items.into()
        }
    }
}
