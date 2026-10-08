use std::fmt::{self, Display};

/// The direction in which a script is written, as used by the HTML `dir`
/// attribute.
///
/// Get it from a language identifier with
/// [`LanguageIdentifier::direction`](crate::LanguageIdentifier::direction).
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     l10n::locale,
///     router::{Slot, layout},
///     view::{View, view},
/// };
///
/// #[layout("/")]
/// async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
///     let locale = locale(cx);
///     Ok(view! {
///         <html lang=(locale.id()) dir=(locale.id().direction())>
///             <body>(slot)</body>
///         </html>
///     })
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Left to right, written `ltr`.
    Ltr,
    /// Right to left, written `rtl`.
    Rtl,
}

impl Direction {
    /// Returns the value of the HTML `dir` attribute for this direction.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        *self.as_promoted_str()
    }

    /// Returns the text held by a `'static` reference, as the view writer's
    /// promoted strings expect.
    pub(crate) const fn as_promoted_str(self) -> &'static &'static str {
        match self {
            Self::Ltr => &"ltr",
            Self::Rtl => &"rtl",
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
