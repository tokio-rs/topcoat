use topcoat::{
    Result,
    view::{Attributes, View, component, view},
};

/// The visual style of a [`button`].
///
/// Mirrors the `variant` options of the shadcn/ui button. [`Default`] is
/// `ButtonVariant::Default`, matching shadcn's `defaultVariants`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// The primary, filled button.
    #[default]
    Default,
    /// A destructive action, such as deleting data.
    Destructive,
    /// A bordered button with a transparent background.
    Outline,
    /// A muted, secondary button.
    Secondary,
    /// A button with no background until hovered.
    Ghost,
    /// A button styled as an inline text link.
    Link,
}

/// The size of a [`button`].
///
/// Mirrors the `size` options of the shadcn/ui button. [`Default`] is
/// `ButtonSize::Default`, matching shadcn's `defaultVariants`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// The standard button height.
    #[default]
    Default,
    /// A compact button.
    Sm,
    /// A larger button.
    Lg,
    /// A square button sized for a single icon.
    Icon,
}

impl ButtonVariant {
    /// The Tailwind classes for this variant.
    fn classes(self) -> &'static str {
        match self {
            Self::Default => "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
            Self::Destructive => {
                "bg-destructive text-white shadow-xs hover:bg-destructive/90 \
                 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 \
                 dark:bg-destructive/60"
            }
            Self::Outline => {
                "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground \
                 dark:bg-input/30 dark:border-input dark:hover:bg-input/50"
            }
            Self::Secondary => {
                "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80"
            }
            Self::Ghost => "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
            Self::Link => "text-primary underline-offset-4 hover:underline",
        }
    }
}

impl ButtonSize {
    /// The Tailwind classes for this size.
    fn classes(self) -> &'static str {
        match self {
            Self::Default => "h-9 px-4 py-2 has-[>svg]:px-3",
            Self::Sm => "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
            Self::Lg => "h-10 rounded-md px-6 has-[>svg]:px-4",
            Self::Icon => "size-9",
        }
    }
}

/// The classes shared by every button, regardless of variant or size.
const BASE: &str = "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md \
    text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 \
    [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 \
    outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] \
    aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 \
    aria-invalid:border-destructive";

/// Builds the full class string for a button of the given `variant` and `size`.
///
/// This is the analogue of shadcn/ui's exported `buttonVariants`. Use it to give
/// button styling to an element that is not a `<button>` — the equivalent of
/// shadcn's `asChild` — such as a link styled as a button:
///
/// ```rust,ignore
/// view! {
///     <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Default))>
///         "Sign in"
///     </a>
/// }
/// ```
pub fn button_variants(variant: ButtonVariant, size: ButtonSize) -> String {
    format!("{BASE} {} {}", variant.classes(), size.classes())
}

/// A button component, mirroring [shadcn/ui's button](https://ui.shadcn.com/docs/components/button).
///
/// The `variant` and `size` parameters select the styling, both defaulting to
/// their `Default` value. Extra `class`es are appended to the computed ones, and
/// any further `attrs` (such as `type`, `disabled`, or event handlers) are
/// forwarded to the underlying `<button>`. Child nodes become the button's
/// content.
///
/// ```rust,ignore
/// view! {
///     button(
///         variant: ButtonVariant::Destructive,
///         attrs: attributes! { type="submit" },
///         "Delete"
///     )
/// }
/// ```
///
/// To style a non-`<button>` element like a button (shadcn's `asChild`), use
/// [`button_variants`] directly.
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
    // shadcn composes classes with `cn` (clsx + tailwind-merge). There is no
    // tailwind-merge here, so caller classes are appended rather than merged:
    // a conflicting utility wins by being last, per the CSS cascade.
    let class = match class.trim() {
        "" => button_variants(variant, size),
        extra => format!("{} {extra}", button_variants(variant, size)),
    };

    view! {
        <button data-slot="button" class=(class) (attrs)>
            (child)
        </button>
    }
}
