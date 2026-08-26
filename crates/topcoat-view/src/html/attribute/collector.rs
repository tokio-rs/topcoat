use std::mem;

use topcoat_core::context::Cx;

use crate::{DynViewPart, Formatter, HtmlContext, ViewHandle};

/// Collects the parts an attribute key or value pushes and turns them into
/// a [`Captured`] type.
///
/// The first part is held as pushed, so a key or value made of a single
/// string is handed over as is. A second part renders both into a `String`
/// that every part after it appends to, so collecting never allocates more
/// than that one `String`.
#[derive(Default)]
pub(crate) struct AttributeCollector {
    state: State,
}

#[derive(Default)]
enum State {
    /// Nothing pushed yet.
    #[default]
    Empty,
    /// The only part pushed so far, held as pushed.
    Single(CollectedPart),
    /// Everything pushed so far, rendered.
    Rendered(String),
}

impl AttributeCollector {
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(crate) fn push(&mut self, cx: &Cx, part: CollectedPart) {
        if matches!(self.state, State::Empty) {
            self.state = State::Single(part);
        } else {
            self.render(cx, |f| part.render(cx, f));
        }
    }

    /// Pushes borrowed text, copying it only while it is the sole part.
    #[inline]
    pub(crate) fn push_str(&mut self, cx: &Cx, value: &str, context: HtmlContext) {
        if matches!(self.state, State::Empty) {
            self.state = State::Single(CollectedPart::String {
                value: value.to_owned(),
                context,
            });
        } else {
            self.render(cx, |f| context.writer(f).write_str(value));
        }
    }

    /// Appends what `write` writes to the rendered form of everything
    /// pushed so far, rendering a held single part first.
    fn render(&mut self, cx: &Cx, write: impl FnOnce(&mut Formatter<'_>)) {
        let mut value = match mem::take(&mut self.state) {
            State::Empty => String::new(),
            State::Single(first) => {
                let mut value = String::new();
                first.render(cx, &mut Formatter::new(&mut value));
                value
            }
            State::Rendered(value) => value,
        };
        write(&mut Formatter::new(&mut value));
        self.state = State::Rendered(value);
    }

    /// Turns the collected parts into a `C`, rendering them with `cx` where
    /// a single string cannot represent them.
    pub(crate) fn finish<C: Captured>(self, cx: &Cx) -> C {
        match self.state {
            State::Empty => C::empty(),
            State::Single(CollectedPart::PromotedStr { value, context }) => {
                C::promoted_str(value, context)
            }
            State::Single(CollectedPart::StaticStr { value, context }) => {
                C::static_str(value, context)
            }
            State::Single(CollectedPart::String { value, context }) => C::string(value, context),
            State::Single(part) => {
                let mut value = String::new();
                part.render(cx, &mut Formatter::new(&mut value));
                C::string(value, HtmlContext::Unescaped)
            }
            State::Rendered(value) => C::string(value, HtmlContext::Unescaped),
        }
    }
}

/// A type an [`AttributeCollector`] finishes into.
pub(crate) trait Captured {
    /// The capture of a key or value that pushed nothing.
    fn empty() -> Self;

    fn promoted_str(value: &'static &'static str, context: HtmlContext) -> Self;

    fn static_str(value: &'static str, context: HtmlContext) -> Self;

    fn string(value: String, context: HtmlContext) -> Self;
}

/// One part pushed into an [`AttributeCollector`].
pub(crate) enum CollectedPart {
    Bool(bool),
    Int(i128),
    Uint(u128),
    F32(f32),
    F64(f64),
    Char {
        value: char,
        context: HtmlContext,
    },
    PromotedStr {
        value: &'static &'static str,
        context: HtmlContext,
    },
    StaticStr {
        value: &'static str,
        context: HtmlContext,
    },
    String {
        value: String,
        context: HtmlContext,
    },
    Dyn {
        part: Box<dyn DynViewPart>,
        context: HtmlContext,
    },
    View(ViewHandle),
}

impl CollectedPart {
    /// Writes the part through `f`, escaped for the context it was pushed
    /// with.
    fn render(self, cx: &Cx, f: &mut Formatter<'_>) {
        use std::fmt::Write;

        match self {
            Self::Bool(value) => f.write_str(if value { "true" } else { "false" }),
            Self::Int(value) => write!(f, "{value}").unwrap(),
            Self::Uint(value) => write!(f, "{value}").unwrap(),
            Self::F32(value) => write!(f, "{value}").unwrap(),
            Self::F64(value) => write!(f, "{value}").unwrap(),
            Self::Char { value, context } => context.writer(f).write_char(value),
            Self::PromotedStr { value, context } => context.writer(f).write_str(value),
            Self::StaticStr { value, context } => context.writer(f).write_str(value),
            Self::String { value, context } => context.writer(f).write_str(&value),
            Self::Dyn { part, context } => part.render(cx, &mut context.writer(f)),
            Self::View(view) => view.render_into(cx, f),
        }
    }
}
