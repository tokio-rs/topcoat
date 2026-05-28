use topcoat_interop::runtime::{Expr, Formatter, Interpreter};
use topcoat_view::runtime::{IntoViewParts, Unescaped, ViewPart};

#[derive(Debug, Clone)]
pub struct BindAttribute<K, V> {
    key: K,
    value: V,
}

impl<K, V> BindAttribute<K, V> {
    #[inline]
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

impl<K, V, T> IntoViewParts for BindAttribute<K, V>
where
    K: IntoViewParts + Clone,
    V: Expr<Output = T>,
    T: Into<String>,
{
    fn into_view_parts(self) -> impl Iterator<Item = ViewPart> {
        let mut js = String::new();
        let mut formatter = Formatter::new(&mut js);
        self.value.fmt_js(&mut formatter);

        let mut interpreter = Interpreter::new();
        let value = self.value.eval(&mut interpreter).into();

        Unescaped::new_unchecked(" ")
            .into_view_parts()
            .chain(self.key.clone().into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(value.into_view_parts())
            .chain(Unescaped::new_unchecked("\" data-topcoat-bind:").into_view_parts())
            .chain(self.key.into_view_parts())
            .chain(Unescaped::new_unchecked("=\"").into_view_parts())
            .chain(js.into_view_parts())
            .chain(Unescaped::new_unchecked("\" ").into_view_parts())
    }
}
