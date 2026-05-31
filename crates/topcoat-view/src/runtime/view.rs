use core::fmt;

use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment, Unescaped};

/// A piece of HTML content.
///
/// A `View` contains a self-contained HTML fragment where all tags are fully
/// closed. This means it can contain multiple sibling elements, but every
/// opened tag must be closed so that the fragment can be safely nested inside
/// a larger HTML document without breaking the surrounding markup.
///
/// ```html
/// <!-- Valid: all tags are closed, safe to nest -->
/// <div>Hello</div>
/// <p>World</p>
///
/// <!-- Invalid: unclosed tag would corrupt the parent document -->
/// <div>Hello
/// ```
///
/// A `View` is inert until [`render`](Self::render) is called: constructing
/// one only stores the underlying [`ViewPart`] tree, with no escaping or
/// string building performed up-front.
#[derive(Debug, Clone)]
pub struct View {
    part: ViewPart,
    size_hint: usize,
}

impl View {
    /// Builds a `View` from any value that can be converted into [`ViewPart`]s.
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
    ///
    /// Walks the underlying [`ViewPart`] tree, invoking
    /// [`Fragment::fmt`](crate::runtime::Fragment::fmt) on each node. The
    /// output buffer is pre-allocated based on
    /// [`Fragment::size_hint`](crate::runtime::Fragment::size_hint), which is
    /// a lower bound, so the buffer may grow during rendering.
    pub fn render(&self, cx: &Cx) -> String {
        let mut buf = String::with_capacity(self.size_hint);
        let mut f = Formatter::new(&mut buf);
        self.fmt(cx, &mut f);
        buf
    }
}

impl Fragment for View {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        self.part.fmt(cx, f);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        self.size_hint
    }
}

/// A single node in the lazy tree backing a [`View`].
///
/// Each variant represents a kind of content the runtime knows how to render
/// without allocating a trait object. Primitive types get dedicated variants
/// so they can be stored inline; arbitrary [`Fragment`] implementations are
/// reached via [`BoxDyn`](Self::BoxDyn), and nested structure is expressed
/// with [`Node`](Self::Node). Like [`View`], `ViewPart`s are inert until
/// rendered.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ViewPart {
    #[non_exhaustive]
    Empty,
    #[non_exhaustive]
    Bool(bool),
    #[non_exhaustive]
    Char(char),
    #[non_exhaustive]
    I8(i8),
    #[non_exhaustive]
    I16(i16),
    #[non_exhaustive]
    I32(i32),
    #[non_exhaustive]
    I64(i64),
    #[non_exhaustive]
    I128(i128),
    #[non_exhaustive]
    Isize(isize),
    #[non_exhaustive]
    U8(u8),
    #[non_exhaustive]
    U16(u16),
    #[non_exhaustive]
    U32(u32),
    #[non_exhaustive]
    U64(u64),
    #[non_exhaustive]
    U128(u128),
    #[non_exhaustive]
    Usize(usize),
    #[non_exhaustive]
    F32(f32),
    #[non_exhaustive]
    F64(f64),
    #[non_exhaustive]
    StaticStr(&'static str),
    #[non_exhaustive]
    String(String),
    #[non_exhaustive]
    UnescapedStaticStr(Unescaped<&'static str>),
    #[non_exhaustive]
    UnescapedString(Unescaped<String>),
    #[non_exhaustive]
    BoxDyn(Box<dyn DynViewPart>),
    #[non_exhaustive]
    Node(Box<[ViewPart]>),
}

impl ViewPart {
    #[inline]
    pub fn empty() -> Self {
        Self::Empty
    }
}

/// Object-safe counterpart to [`Fragment`] used by [`ViewPart::BoxDyn`].
///
/// Allows arbitrary [`Fragment`] implementations to be stored inside a
/// [`ViewPart`] behind a `Box<dyn ...>`. A blanket impl covers every type
/// that is `Fragment + Debug + Clone + Send + 'static`, so user code should
/// rarely need to implement this trait directly.
pub trait DynViewPart: 'static + Fragment + fmt::Debug + Send {
    /// Clones the underlying value into a fresh `Box<dyn DynViewPart>`.
    ///
    /// Required because `dyn DynViewPart` cannot use the standard `Clone`
    /// trait directly.
    fn clone_box(&self) -> Box<dyn DynViewPart>;
}

impl<T> DynViewPart for T
where
    T: 'static + Fragment + fmt::Debug + Clone + Send,
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

impl Fragment for ViewPart {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        match self {
            Self::Empty => {}
            Self::Bool(inner) => inner.fmt(cx, f),
            Self::Char(inner) => inner.fmt(cx, f),
            Self::I8(inner) => inner.fmt(cx, f),
            Self::I16(inner) => inner.fmt(cx, f),
            Self::I32(inner) => inner.fmt(cx, f),
            Self::I64(inner) => inner.fmt(cx, f),
            Self::I128(inner) => inner.fmt(cx, f),
            Self::Isize(inner) => inner.fmt(cx, f),
            Self::U8(inner) => inner.fmt(cx, f),
            Self::U16(inner) => inner.fmt(cx, f),
            Self::U32(inner) => inner.fmt(cx, f),
            Self::U64(inner) => inner.fmt(cx, f),
            Self::U128(inner) => inner.fmt(cx, f),
            Self::Usize(inner) => inner.fmt(cx, f),
            Self::F32(inner) => inner.fmt(cx, f),
            Self::F64(inner) => inner.fmt(cx, f),
            Self::String(inner) => inner.fmt(cx, f),
            Self::StaticStr(inner) => inner.fmt(cx, f),
            Self::UnescapedString(inner) => inner.fmt(cx, f),
            Self::UnescapedStaticStr(inner) => inner.fmt(cx, f),
            Self::BoxDyn(inner) => Fragment::fmt(inner, cx, f),
            Self::Node(inner) => {
                for part in inner.iter() {
                    part.fmt(cx, f);
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
            Self::BoxDyn(inner) => Fragment::size_hint(inner),
            Self::Node(inner) => inner.iter().map(|part| part.size_hint()).sum(),
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
    Node(Box<[ViewPart]>),
}

impl From<View> for ViewPart {
    fn from(value: View) -> Self {
        value.part
    }
}

#[derive(Debug, Default, Clone)]
pub struct ViewParts {
    first: Option<ViewPart>,
    items: Vec<ViewPart>,
}

impl ViewParts {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

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
            value.items.into_boxed_slice().into()
        }
    }
}
