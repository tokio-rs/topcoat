use std::fmt::{self, Display};

/// The writing direction of a script, suitable for the HTML `dir` attribute.
///
/// Use [`LanguageIdentifier::direction`](crate::LanguageIdentifier::direction) to get
/// it from a language identifier.
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
///     let id = locale(cx).id();
///     Ok(view! {
///         <html lang=(id) dir=(id.direction())>
///             <body>(slot)</body>
///         </html>
///     })
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Text runs from left to right. Displays as `ltr`.
    Ltr,
    /// Text runs from right to left. Displays as `rtl`.
    Rtl,
}

impl Direction {
    /// Returns `ltr` or `rtl` for an HTML `dir` attribute.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.as_promoted_str()
    }

    /// Returns a static reference to the direction text for the view writer.
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
