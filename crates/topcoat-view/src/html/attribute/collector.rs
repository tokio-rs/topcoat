use topcoat_core::context::Cx;

use crate::{DynViewPart, Formatter, HtmlContext, ViewHandle};

/// Collects the parts an attribute key or value pushes and turns them into
/// a [`Captured`] type.
///
/// A single string part is handed over as is; any other combination is
/// rendered into a `String` when the collector finishes.
#[derive(Default)]
pub(crate) struct AttributeCollector {
    first: Option<CollectedPart>,
    rest: Vec<CollectedPart>,
}

impl AttributeCollector {
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(crate) fn push(&mut self, part: CollectedPart) {
        if self.first.is_none() {
            self.first = Some(part);
        } else {
            self.rest.push(part);
        }
    }

    /// Turns the collected parts into a `C`, rendering them with `cx` where
    /// a single string cannot represent them.
    pub(crate) fn finish<C: Captured>(self, cx: &Cx) -> C {
        let Some(first) = self.first else {
            return C::empty();
        };
        let first = match (first, self.rest.is_empty()) {
            (CollectedPart::PromotedStr { value, context }, true) => {
                return C::promoted_str(value, context);
            }
            (CollectedPart::StaticStr { value, context }, true) => {
                return C::static_str(value, context);
            }
            (CollectedPart::String { value, context }, true) => {
                return C::string(value, context);
            }
            (first, _) => first,
        };
        let mut value = String::new();
        let mut f = Formatter::new(&mut value);
        first.render(cx, &mut f);
        for part in self.rest {
            part.render(cx, &mut f);
        }
        C::string(value, HtmlContext::Unescaped)
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
