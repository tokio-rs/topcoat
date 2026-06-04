use core::fmt;

use topcoat_core::context::Cx;

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
#[derive(Debug, Clone)]
pub struct View {
    part: ViewPart,
    size_hint: usize,
}

impl View {
    /// Creates a view from accumulated view parts.
    ///
    /// This is usually called by generated `view!` code after collecting the
    /// nodes and attributes for a fragment.
    #[inline]
    pub fn new(parts: ViewParts) -> Self {
        Self {
            part: parts.into(),
            size_hint: 0,
        }
    }

    /// Returns a `View` that renders to an empty string.
    #[inline]
    pub fn empty() -> Self {
        Self {
            part: ViewPart::Empty,
            size_hint: 0,
        }
    }

    /// Renders the view into an HTML string.
    pub fn render(&self, cx: &Cx) -> String {
        let mut buf = String::with_capacity(self.size_hint);
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
        self.size_hint
    }
}

/// A renderable value stored in a [`View`].
///
/// Most code creates view parts through [`ViewParts::push`] or the `view!`
/// macro rather than constructing enum variants directly.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ViewPart {
    /// Renders no content.
    #[non_exhaustive]
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
    BoxDyn(Box<dyn DynViewPart>),
    /// A sequence of view parts rendered in order.
    #[non_exhaustive]
    BoxSlice(Box<[ViewPart]>),
    /// A Vec of view parts rendered in order.
    #[non_exhaustive]
    Vec(Vec<ViewPart>),
}

impl ViewPart {
    /// Returns an empty view part.
    #[inline]
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
            Self::BoxDyn(inner) => FmtHtml::fmt_html(inner, cx, f),
            Self::BoxSlice(inner) => {
                for part in inner.iter() {
                    part.fmt_html(cx, f);
                }
            }
            Self::Vec(inner) => {
                for part in inner.iter() {
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
            Self::BoxDyn(inner) => FmtHtml::size_hint(inner),
            Self::BoxSlice(inner) => inner.iter().map(|part| part.size_hint()).sum(),
            Self::Vec(inner) => inner.iter().map(|part| part.size_hint()).sum(),
        }
    }
}

macro_rules! impl_from_for_view_part {
    ($($variant:ident($ty:ty)),* $(,)?) => {
        $(
            impl From<$ty> for ViewPart {
                #[inline]
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

impl_from_for_view_part! {
    Bool(bool),
    Char(char),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F32(f32),
    F64(f64),
    StaticStr(&'static str),
    String(String),
    UnescapedStaticStr(Unescaped<&'static str>),
    UnescapedString(Unescaped<String>),
    BoxDyn(Box<dyn DynViewPart>),
    BoxSlice(Box<[ViewPart]>),
    Vec(Vec<ViewPart>),
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
            self.first = Some(part)
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
