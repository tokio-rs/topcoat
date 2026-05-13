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
#[derive(Debug, Clone)]
pub struct View {
    part: ViewPart,
}

impl View {
    #[inline]
    pub fn new(part: impl IntoViewPart) -> Self {
        Self {
            part: part.into_view_part(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self::new(ViewPart::Empty)
    }

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
    String(String),
    UnescapedString(Unescaped<String>),
    BoxDyn(Box<dyn DynViewPart>),
    Node(Box<[ViewPart]>),
}

pub trait DynViewPart: fmt::Debug + Send {
    fn dyn_fmt(&self, cx: &Cx, f: &mut Formatter<'_>);
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
            Self::Bool(v) => v.fmt(cx, f),
            Self::Char(v) => v.fmt(cx, f),
            Self::I8(v) => v.fmt(cx, f),
            Self::I16(v) => v.fmt(cx, f),
            Self::I32(v) => v.fmt(cx, f),
            Self::I64(v) => v.fmt(cx, f),
            Self::I128(v) => v.fmt(cx, f),
            Self::Isize(v) => v.fmt(cx, f),
            Self::U8(v) => v.fmt(cx, f),
            Self::U16(v) => v.fmt(cx, f),
            Self::U32(v) => v.fmt(cx, f),
            Self::U64(v) => v.fmt(cx, f),
            Self::U128(v) => v.fmt(cx, f),
            Self::Usize(v) => v.fmt(cx, f),
            Self::F32(v) => v.fmt(cx, f),
            Self::F64(v) => v.fmt(cx, f),
            Self::String(s) => s.fmt(cx, f),
            Self::UnescapedString(s) => s.fmt(cx, f),
            Self::BoxDyn(d) => d.dyn_fmt(cx, f),
            Self::Node(parts) => {
                for part in parts.iter() {
                    part.fmt(cx, f);
                }
            }
        }
    }

    fn size_hint(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Bool(v) => v.size_hint(),
            Self::Char(v) => v.size_hint(),
            Self::I8(v) => v.size_hint(),
            Self::I16(v) => v.size_hint(),
            Self::I32(v) => v.size_hint(),
            Self::I64(v) => v.size_hint(),
            Self::I128(v) => v.size_hint(),
            Self::Isize(v) => v.size_hint(),
            Self::U8(v) => v.size_hint(),
            Self::U16(v) => v.size_hint(),
            Self::U32(v) => v.size_hint(),
            Self::U64(v) => v.size_hint(),
            Self::U128(v) => v.size_hint(),
            Self::Usize(v) => v.size_hint(),
            Self::F32(v) => v.size_hint(),
            Self::F64(v) => v.size_hint(),
            Self::String(s) => s.size_hint(),
            Self::UnescapedString(s) => s.len(),
            Self::BoxDyn(_) => 0,
            Self::Node(parts) => parts.iter().map(|part| part.size_hint()).sum(),
        }
    }
}

pub trait IntoViewPart {
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
