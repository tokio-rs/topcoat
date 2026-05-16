use core::fmt;

use topcoat_core::context::Cx;

use crate::runtime::{Attribute, Formatter, Fragment, Unescaped};

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
}

impl View {
    /// Builds a `View` from any value that can be converted into a [`ViewPart`].
    #[inline]
    pub fn new(part: impl IntoViewPart) -> Self {
        Self {
            part: part.into_view_part(),
        }
    }

    /// Returns a `View` that renders to an empty string.
    #[inline]
    pub fn empty() -> Self {
        Self::new(ViewPart::Empty)
    }

    /// Renders the view into an HTML string.
    ///
    /// Walks the underlying [`ViewPart`] tree, invoking
    /// [`Fragment::fmt`](crate::runtime::Fragment::fmt) on each node. The
    /// output buffer is pre-allocated based on
    /// [`Fragment::size_hint`](crate::runtime::Fragment::size_hint), which is
    /// a lower bound, so the buffer may grow during rendering.
    pub fn render(&self, cx: &Cx) -> String {
        let mut buf = String::with_capacity(self.size_hint());
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
        self.part.size_hint()
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
pub enum ViewPart {
    Empty,
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
    Attribute(Box<Attribute>),
    BoxDyn(Box<dyn DynViewPart>),
    Node(Box<[ViewPart]>),
}

/// Object-safe counterpart to [`Fragment`] used by [`ViewPart::BoxDyn`].
///
/// Allows arbitrary [`Fragment`] implementations to be stored inside a
/// [`ViewPart`] behind a `Box<dyn ...>`. A blanket impl covers every type
/// that is `Fragment + Debug + Clone + Send + 'static`, so user code should
/// rarely need to implement this trait directly.
pub trait DynViewPart: fmt::Debug + Send {
    /// Renders the underlying fragment. See [`Fragment::fmt`].
    fn dyn_fmt(&self, cx: &Cx, f: &mut Formatter<'_>);

    /// Returns the size hint of the underlying fragment. See
    /// [`Fragment::size_hint`].
    fn dyn_size_hint(&self) -> usize;

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
    fn dyn_fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        Fragment::fmt(self, cx, f);
    }

    #[inline]
    fn dyn_size_hint(&self) -> usize {
        Fragment::size_hint(self)
    }

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
            Self::Attribute(inner) => inner.fmt(cx, f),
            Self::BoxDyn(inner) => inner.dyn_fmt(cx, f),
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
            Self::Attribute(inner) => inner.size_hint(),
            Self::BoxDyn(inner) => inner.dyn_size_hint(),
            Self::Node(inner) => inner.iter().map(|part| part.size_hint()).sum(),
        }
    }
}

/// Conversion into a [`ViewPart`].
///
/// The `view!` macro calls `into_view_part` on every expression interpolated
/// into the view tree, so implementing this trait is what makes a type
/// embeddable inside `view!` markup. Built-in impls cover primitives,
/// strings, [`Option`], and pre-built [`View`]/[`ViewPart`] values; user
/// types can opt in by adding their own impl.
pub trait IntoViewPart {
    /// Consumes `self` and produces the corresponding [`ViewPart`].
    fn into_view_part(self) -> ViewPart;
}

impl IntoViewPart for View {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        self.part
    }
}

impl IntoViewPart for ViewPart {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        self
    }
}

impl<T> IntoViewPart for &T
where
    T: IntoViewPart + Copy,
{
    fn into_view_part(self) -> ViewPart {
        (*self).into_view_part()
    }
}

impl IntoViewPart for &str {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::String(self.to_owned())
    }
}

impl IntoViewPart for String {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::String(self)
    }
}

impl IntoViewPart for Box<dyn DynViewPart> {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::BoxDyn(self)
    }
}

impl<const N: usize> IntoViewPart for [ViewPart; N] {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::Node(Box::new(self))
    }
}

impl IntoViewPart for Box<[ViewPart]> {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::Node(self)
    }
}

impl<const N: usize> IntoViewPart for Box<[ViewPart; N]> {
    #[inline]
    fn into_view_part(self) -> ViewPart {
        ViewPart::Node(self)
    }
}

macro_rules! impl_into_view_part_primitive {
    ($variant:ident, $ty:ty) => {
        impl IntoViewPart for $ty {
            #[inline]
            fn into_view_part(self) -> ViewPart {
                ViewPart::$variant(self)
            }
        }
    };
}

impl_into_view_part_primitive!(Bool, bool);
impl_into_view_part_primitive!(Char, char);
impl_into_view_part_primitive!(I8, i8);
impl_into_view_part_primitive!(I16, i16);
impl_into_view_part_primitive!(I32, i32);
impl_into_view_part_primitive!(I64, i64);
impl_into_view_part_primitive!(I128, i128);
impl_into_view_part_primitive!(Isize, isize);
impl_into_view_part_primitive!(U8, u8);
impl_into_view_part_primitive!(U16, u16);
impl_into_view_part_primitive!(U32, u32);
impl_into_view_part_primitive!(U64, u64);
impl_into_view_part_primitive!(U128, u128);
impl_into_view_part_primitive!(Usize, usize);
impl_into_view_part_primitive!(F32, f32);
impl_into_view_part_primitive!(F64, f64);

impl<T> IntoViewPart for Option<T>
where
    T: IntoViewPart,
{
    fn into_view_part(self) -> ViewPart {
        match self {
            Some(value) => value.into_view_part(),
            None => ViewPart::Empty,
        }
    }
}
