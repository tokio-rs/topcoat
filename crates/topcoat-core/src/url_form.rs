//! The form URLs render in: relative to the site root or absolute.

use crate::context::{Cx, try_request_context};

/// The form a URL renders in when the code writing it does not choose one.
///
/// Relative URLs work anywhere within the site, while rendered content that
/// leaves it (e.g. links and images in emails, feeds, or sitemaps) needs the
/// absolute form. The form in effect is a request context value, read with
/// [`url_form`]: URLs render relative unless an enclosing scope registers the
/// absolute form with [`Cx::with`].
///
/// ```
/// use topcoat::context::{Cx, UrlForm, url_form};
///
/// let cx = Cx::default();
/// assert_eq!(url_form(&cx), UrlForm::Relative);
///
/// let cx = cx.with(UrlForm::Absolute);
/// assert_eq!(url_form(&cx), UrlForm::Absolute);
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum UrlForm {
    /// The URL renders relative to the site root, like `/posts/42`.
    #[default]
    Relative,
    /// The URL renders with its scheme and host, like
    /// `https://example.com/posts/42`.
    Absolute,
}

/// Returns the [`UrlForm`] in effect for this context.
///
/// URLs render [`Relative`](UrlForm::Relative) unless an enclosing scope
/// registered another form on the request context.
#[inline]
#[must_use]
pub fn url_form(cx: &Cx) -> UrlForm {
    try_request_context::<UrlForm>(cx).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_relative() {
        assert_eq!(url_form(&Cx::default()), UrlForm::Relative);
    }

    #[test]
    fn a_scope_registers_another_form() {
        let cx = Cx::default().with(UrlForm::Absolute);
        assert_eq!(url_form(&cx), UrlForm::Absolute);
    }

    #[test]
    fn an_inner_scope_shadows_the_outer_form() {
        let cx = Cx::default().with(UrlForm::Absolute);
        let inner = cx.with(UrlForm::Relative);
        assert_eq!(url_form(&inner), UrlForm::Relative);
        assert_eq!(url_form(&cx), UrlForm::Absolute);
    }
}
