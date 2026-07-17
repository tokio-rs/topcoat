#[cfg(feature = "http")]
use http::{HeaderMap, StatusCode};

/// A plain string writer that render output accumulates into.
///
/// `Formatter` is escaping-agnostic: [`write_str`](Self::write_str) and
/// [`write_char`](Self::write_char) append exactly what they are given. Text
/// that needs to be made safe for an HTML position is written through an
/// [`HtmlWriter`](crate::HtmlWriter) created for the matching
/// [`HtmlContext`](crate::HtmlContext) instead.
pub struct Formatter<'a> {
    buf: &'a mut String,
    #[cfg(feature = "http")]
    status_code: Option<StatusCode>,
    #[cfg(feature = "http")]
    headers: HeaderMap,
}

impl<'a> Formatter<'a> {
    /// Creates a new `Formatter` that writes into the given destination.
    #[inline]
    pub fn new(buf: &'a mut String) -> Self {
        Self {
            buf,
            #[cfg(feature = "http")]
            status_code: None,
            #[cfg(feature = "http")]
            headers: HeaderMap::new(),
        }
    }

    /// Writes a string verbatim.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Writes a single character verbatim.
    #[inline]
    pub fn write_char(&mut self, c: char) {
        self.buf.push(c);
    }

    /// Records a response status code, keeping an earlier one if already
    /// recorded: the first status code rendered wins.
    #[cfg(feature = "http")]
    #[inline]
    pub(crate) fn record_status_code(&mut self, status_code: StatusCode) {
        self.status_code.get_or_insert(status_code);
    }

    /// Records response headers, keeping earlier values for names already
    /// recorded: the first render part that mentions a header name provides
    /// all of that name's values.
    #[cfg(feature = "http")]
    pub(crate) fn record_headers(&mut self, headers: &HeaderMap) {
        if self.headers.is_empty() {
            self.headers = headers.clone();
            return;
        }
        for name in headers.keys() {
            if !self.headers.contains_key(name) {
                for value in headers.get_all(name) {
                    self.headers.append(name.clone(), value.clone());
                }
            }
        }
    }

    /// Consumes the formatter, returning the recorded status code and
    /// headers.
    #[cfg(feature = "http")]
    pub(crate) fn into_recorded(self) -> (Option<StatusCode>, HeaderMap) {
        (self.status_code, self.headers)
    }
}

impl std::fmt::Write for Formatter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Formatter::write_str(self, s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        Formatter::write_char(self, c);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_str_is_verbatim() {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_str("<b>&\"'</b>");
        assert_eq!(buf, "<b>&\"'</b>");
    }

    #[test]
    fn write_char_is_verbatim() {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_char('<');
        f.write_char('é');
        assert_eq!(buf, "<é");
    }

    #[test]
    fn fmt_write_is_verbatim() {
        use std::fmt::Write;

        let (one, two) = (1, "two");
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        write!(f, "{one} < {two}").unwrap();
        assert_eq!(buf, "1 < two");
    }
}
