use topcoat_view::runtime::ViewParts;

pub trait JsViewParts {
    fn to_view_parts(&self, parts: &mut ViewParts);
}

impl JsViewParts for str {
    fn to_view_parts(&self, parts: &mut ViewParts) {
        parts.push(self.to_owned());
    }
}

impl<T: JsViewParts> JsViewParts for &T
where
    T: ?Sized,
{
    fn to_view_parts(&self, parts: &mut ViewParts) {
        <T as JsViewParts>::to_view_parts(*self, parts);
    }
}
