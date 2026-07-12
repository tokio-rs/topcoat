use topcoat::{
    Result,
    view::{Attributes, View, component, view},
};

/// The visual style of a [`button`].
///
/// [`Default`] is `ButtonVariant::Primary`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    /// The primary-filled button for the main action.
    #[default]
    Primary,
    /// A muted, tinted fill for secondary actions.
    Secondary,
    /// A hairline-bordered button on the page background.
    Outline,
    /// No fill until hovered, for toolbars and inline actions.
    Ghost,
    /// A destructive-filled button for actions such as deleting data.
    Destructive,
}

impl ButtonVariant {
    /// The Tailwind classes for this variant.
    ///
    /// Hover and press states apply the fill or foreground color at reduced
    /// opacity, so they hold up in both color schemes without `dark:`
    /// overrides. Every variant with a resting fill or border casts the
    /// theme's control shadow; `Ghost` is flat until hovered, so it casts
    /// none.
    fn classes(self) -> &'static str {
        match self {
            Self::Primary => {
                "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90 \
                 active:bg-primary/80"
            }
            Self::Secondary => {
                "bg-foreground/5 text-foreground shadow-xs hover:bg-foreground/10 \
                 active:bg-foreground/15"
            }
            Self::Outline => {
                "border-border text-foreground shadow-xs hover:bg-foreground/5 \
                 active:bg-foreground/10"
            }
            Self::Ghost => "text-foreground hover:bg-foreground/5 active:bg-foreground/10",
            Self::Destructive => {
                "bg-destructive text-destructive-foreground shadow-xs hover:bg-destructive/90 \
                 active:bg-destructive/80"
            }
        }
    }
}

/// The size of a [`button`].
///
/// [`Default`] is `ButtonSize::Md`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonSize {
    /// A compact button.
    Sm,
    /// The standard button size.
    #[default]
    Md,
    /// A prominent button.
    Lg,
    /// A square button sized for a single icon.
    Icon,
}

impl ButtonSize {
    /// The Tailwind classes for this size.
    ///
    /// Each size sets a text size, which also scales any icons inside: the
    /// `icon` component is `1em` square by default.
    fn classes(self) -> &'static str {
        match self {
            Self::Sm => "h-8 gap-1.5 rounded-md px-3 text-xs",
            Self::Md => "h-9 gap-2 rounded-lg px-4 text-sm",
            Self::Lg => "h-10 gap-2 rounded-lg px-5 text-base",
            Self::Icon => "size-9 rounded-lg text-base",
        }
    }
}

/// The classes shared by every button, regardless of variant or size.
///
/// Every button carries a transparent border so that the `Outline` variant,
/// which only recolors it, does not change the button's dimensions.
const BASE: &str = "inline-flex shrink-0 items-center justify-center border border-transparent \
    font-medium whitespace-nowrap transition-colors outline-none select-none \
    focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
    focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

/// Builds the full class string for a button of the given `variant` and `size`.
///
/// Use it to give button styling to an element that is not a `<button>`, such
/// as a link styled as a button:
///
/// ```rust
/// view! {
///     <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
///         "Sign in"
///     </a>
/// }
/// ```
#[must_use]
pub fn button_variants(variant: ButtonVariant, size: ButtonSize) -> String {
    format!("{BASE} {} {}", variant.classes(), size.classes())
}

/// A button component.
///
/// The `variant` and `size` parameters select the styling, defaulting to
/// `Primary` and `Md`. Extra `class`es are appended to the computed ones, and
/// any further `attrs` (such as `type`, `disabled`, or event handlers) are
/// forwarded to the underlying `<button>`. Child nodes become the button's
/// content.
///
/// ```rust
/// view! {
///     button(
///         variant: ButtonVariant::Destructive,
///         attrs: attributes! { type="submit" },
///         "Delete"
///     )
/// }
/// ```
///
/// To style a non-`<button>` element like a button, use [`button_variants`]
/// directly.
#[component]
pub async fn button(
    #[default] variant: ButtonVariant,
    #[default] size: ButtonSize,
    #[into]
    #[default]
    class: String,
    #[default] attrs: Attributes,
    #[default] child: View,
) -> Result {
    // There is no tailwind-merge here, so caller classes are appended rather
    // than merged: a conflicting utility wins by being last, per the CSS
    // cascade.
    let class = match class.trim() {
        "" => button_variants(variant, size),
        extra => format!("{} {extra}", button_variants(variant, size)),
    };

    view! { <button class=(class) (attrs)>(child)</button> }
}
