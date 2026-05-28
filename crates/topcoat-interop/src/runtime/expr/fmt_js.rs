/// An expression that can be formatted as JavaScript source code to be run in the browser.
pub trait FmtJs {
    fn fmt_js(&self, f: &mut Formatter<'_>);
}

pub struct Formatter<'a> {
    buf: &'a mut String,
}

impl<'a> Formatter<'a> {
    pub fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }

    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    #[inline]
    pub fn write_char(&mut self, ch: char) {
        self.buf.push(ch);
    }
}
