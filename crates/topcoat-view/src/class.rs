use std::borrow::Cow;

use topcoat_core::context::Cx;

use crate::{AttributeValueViewParts, PartsWriter};

/// Converts a value used as a class list entry into view parts.
///
/// When this trait is implemented on a type, it can be used as an entry in
/// the [`class!`](https://docs.rs/topcoat/latest/topcoat/view/macro.class.html)
/// macro or stored in a [`Class`] directly.
///
/// A class list separates its entries with single spaces. An absent entry
/// must not produce a separator, so [`is_present`](Self::is_present) is the
/// hook that makes that decision: the built-in `Option<T>` implementation
/// reports `None` as absent, and the string implementations report empty
/// strings as absent.
pub trait ClassViewParts {
    /// Returns whether this value contributes to the class list.
    ///
    /// An absent value is skipped entirely and does not produce a separating
    /// space.
    fn is_present(&self) -> bool;

    /// Appends this value to the class list being built.
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>);
}

impl ClassViewParts for &str {
    #[inline]
    fn is_present(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self.to_owned());
    }
}

impl ClassViewParts for String {
    #[inline]
    fn is_present(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self);
    }
}

impl ClassViewParts for &String {
    #[inline]
    fn is_present(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        ClassViewParts::into_view_parts(self.as_str(), cx, parts);
    }
}

impl ClassViewParts for Cow<'static, str> {
    #[inline]
    fn is_present(&self) -> bool {
        !self.is_empty()
    }

    #[inline]
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_str(self);
    }
}

impl<T> ClassViewParts for Option<T>
where
    T: ClassViewParts,
{
    #[inline]
    fn is_present(&self) -> bool {
        self.as_ref().is_some_and(T::is_present)
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(value) = self {
            value.into_view_parts(cx, parts);
        }
    }
}

/// A conditional class list entry holding whichever branch was taken.
///
/// The `class!` macro lowers an `if`/`else` entry to this enum when the two
/// branches have different types.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub enum ClassBranch<A, B> {
    Then(A),
    Else(B),
}

impl<A, B> ClassViewParts for ClassBranch<A, B>
where
    A: ClassViewParts,
    B: ClassViewParts,
{
    #[inline]
    fn is_present(&self) -> bool {
        match self {
            Self::Then(inner) => inner.is_present(),
            Self::Else(inner) => inner.is_present(),
        }
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self {
            Self::Then(inner) => inner.into_view_parts(cx, parts),
            Self::Else(inner) => inner.into_view_parts(cx, parts),
        }
    }
}

/// A writer that separates class list entries with single spaces.
///
/// [`Class`] creates one per class list when the attribute value is emitted
/// and passes it to [`ClassEntries::write_entries`]. Absent entries are
/// skipped without producing a separator.
pub struct ClassWriter<'a, 'b> {
    parts: &'b mut PartsWriter<'a>,
    first: bool,
}

impl<'a, 'b> ClassWriter<'a, 'b> {
    /// Creates a writer over `parts` with no entries written yet.
    #[inline]
    pub(crate) fn new(parts: &'b mut PartsWriter<'a>) -> Self {
        Self { parts, first: true }
    }

    /// Appends an entry, separated from the previous entry by a single
    /// space.
    ///
    /// An absent entry (for example [`None`] or an empty string) is skipped
    /// entirely.
    #[inline]
    pub fn entry(&mut self, cx: &Cx, value: impl ClassViewParts) -> &mut Self {
        if value.is_present() {
            if !self.first {
                self.parts.push_str(" ");
            }
            value.into_view_parts(cx, self.parts);
            self.first = false;
        }
        self
    }
}

/// One or more class list entries written through a [`ClassWriter`].
///
/// This is the bound [`Class`] places on its contents. It is implemented for
/// every [`ClassViewParts`] value, for tuples of entries, and for arrays and
/// [`Vec`]s of entries, so a class list holds a mix of static and dynamic
/// entries inline without allocating for itself.
pub trait ClassEntries {
    /// Returns whether any entry contributes to the class list.
    fn any_present(&self) -> bool;

    /// Writes every entry through `writer`.
    fn write_entries(self, cx: &Cx, writer: &mut ClassWriter<'_, '_>);
}

impl<T> ClassEntries for T
where
    T: ClassViewParts,
{
    #[inline]
    fn any_present(&self) -> bool {
        self.is_present()
    }

    #[inline]
    fn write_entries(self, cx: &Cx, writer: &mut ClassWriter<'_, '_>) {
        writer.entry(cx, self);
    }
}

impl ClassEntries for () {
    #[inline]
    fn any_present(&self) -> bool {
        false
    }

    #[inline]
    fn write_entries(self, _cx: &Cx, _writer: &mut ClassWriter<'_, '_>) {}
}

impl<T> ClassEntries for Vec<T>
where
    T: ClassEntries,
{
    #[inline]
    fn any_present(&self) -> bool {
        self.iter().any(ClassEntries::any_present)
    }

    #[inline]
    fn write_entries(self, cx: &Cx, writer: &mut ClassWriter<'_, '_>) {
        for entry in self {
            entry.write_entries(cx, writer);
        }
    }
}

impl<T, const N: usize> ClassEntries for [T; N]
where
    T: ClassEntries,
{
    #[inline]
    fn any_present(&self) -> bool {
        self.iter().any(ClassEntries::any_present)
    }

    #[inline]
    fn write_entries(self, cx: &Cx, writer: &mut ClassWriter<'_, '_>) {
        for entry in self {
            entry.write_entries(cx, writer);
        }
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),+) => {
        impl<$($ty),+> ClassEntries for ($($ty,)+)
        where
            $($ty: ClassEntries,)+
        {
            #[inline]
            #[allow(non_snake_case)]
            fn any_present(&self) -> bool {
                let ($($ty,)+) = self;
                $($ty.any_present())||+
            }

            #[inline]
            #[allow(non_snake_case)]
            fn write_entries(self, cx: &Cx, writer: &mut ClassWriter<'_, '_>) {
                let ($($ty,)+) = self;
                $($ty.write_entries(cx, writer);)+
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

/// A space-separated list of HTML classes.
///
/// Prefer constructing `Class` with the [`class!`](../view/macro.class.html)
/// macro. The entries live inline in the value (a single entry, a tuple, an
/// array, or a [`Vec`]), so building a class list performs no allocation of
/// its own; the entries are written directly into the surrounding view when
/// the attribute value is emitted.
///
/// A `Class` is used in the attribute value position of an element, where a
/// class list without present entries omits the whole attribute:
///
/// ```rust
/// # use topcoat::view::{class, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// # let is_active = true;
/// view! {
///     <button class=(class!("btn", "active" if is_active))>"Save"</button>
/// }
/// # }
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct Class<T>(pub T);

impl<T> AttributeValueViewParts for Class<T>
where
    T: ClassEntries,
{
    #[inline]
    fn attribute_present(&self) -> bool {
        self.0.any_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.0.write_entries(cx, &mut ClassWriter::new(parts));
    }
}

impl<T> ClassViewParts for Class<T>
where
    T: ClassEntries,
{
    #[inline]
    fn is_present(&self) -> bool {
        self.0.any_present()
    }

    #[inline]
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        // The surrounding class list has already written any separator for
        // this entry, so the nested list starts fresh and only separates its
        // own entries.
        self.0.write_entries(cx, &mut ClassWriter::new(parts));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HtmlContext, View, ViewParts};

    fn render(class: Class<impl ClassEntries>) -> String {
        let cx = Cx::default();
        let mut parts = ViewParts::new();
        AttributeValueViewParts::into_view_parts(
            class,
            &cx,
            &mut PartsWriter::new(&mut parts, HtmlContext::AttributeValue),
        );
        View::new(parts).render(&cx)
    }

    #[test]
    fn no_entries_is_absent() {
        let class = Class(());
        assert!(!class.attribute_present());
        assert_eq!(render(class), "");
    }

    #[test]
    fn single_entry_renders_without_separator() {
        let class = Class("btn");
        assert!(class.attribute_present());
        assert_eq!(render(class), "btn");
    }

    #[test]
    fn entries_are_separated_by_single_spaces() {
        let class = Class(("btn", "btn-lg".to_owned(), Cow::Borrowed("active")));
        assert_eq!(render(class), "btn btn-lg active");
    }

    #[test]
    fn none_option_is_skipped_without_a_separator() {
        let class = Class(("a", Option::<&str>::None, "b"));
        assert_eq!(render(class), "a b");
    }

    #[test]
    fn some_option_is_rendered() {
        assert_eq!(render(Class(Some("active"))), "active");
    }

    #[test]
    fn empty_strings_are_skipped_without_a_separator() {
        let class = Class(("a", "", String::new(), Some(""), "b"));
        assert_eq!(render(class), "a b");
    }

    #[test]
    fn all_entries_absent_omits_the_attribute() {
        let class = Class(("", Option::<&str>::None));
        assert!(!class.attribute_present());
        assert_eq!(render(class), "");
    }

    #[test]
    fn skipped_leading_entry_produces_no_leading_space() {
        let class = Class((Option::<&str>::None, "a"));
        assert_eq!(render(class), "a");
    }

    #[test]
    fn entries_are_escaped_for_the_attribute_value_position() {
        assert_eq!(render(Class("a\"b")), "a&quot;b");
    }

    #[test]
    fn branch_entries_render_the_taken_branch() {
        let class = Class((
            ClassBranch::<&str, String>::Then("on"),
            ClassBranch::<&str, String>::Else("off".to_owned()),
        ));
        assert_eq!(render(class), "on off");
    }

    #[test]
    fn nested_class_is_spliced_with_separators() {
        let class = Class(("card", Class(("btn", "btn-lg"))));
        assert_eq!(render(class), "card btn btn-lg");
    }

    #[test]
    fn empty_nested_class_is_skipped_without_a_separator() {
        let class = Class(("a", Class(()), "b"));
        assert_eq!(render(class), "a b");
    }

    #[test]
    fn nested_tuples_flatten_with_separators() {
        let class = Class((("a", "b"), ("c", "d")));
        assert_eq!(render(class), "a b c d");
    }

    #[test]
    fn vec_entries_render_with_separators() {
        let class = Class(vec!["a".to_owned(), String::new(), "b".to_owned()]);
        assert_eq!(render(class), "a b");
    }

    #[test]
    fn array_entries_render_with_separators() {
        let class = Class([Some("a"), None, Some("b")]);
        assert_eq!(render(class), "a b");
    }

    #[test]
    fn borrowed_cow_entries_render_without_reallocating() {
        let class = Class((
            Cow::Borrowed("static"),
            Cow::<'static, str>::Owned("owned".to_owned()),
        ));
        assert_eq!(render(class), "static owned");
    }
}
