use topcoat::{
    Result,
    view::{Attributes, View, component, view},
};

/// The visual style of a [`button`].
///
/// [`Default`] is `ButtonVariant::Default`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
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

impl ButtonVariant {
    /// The string key for this variant, emitted as `data-variant`.
    fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Secondary => "secondary",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }

    /// The Tailwind classes for this variant.
    fn classes(self) -> &'static str {
        match self {
            Self::Default => "bg-primary text-primary-foreground hover:bg-primary/80",
            Self::Destructive => {
                "bg-destructive/10 text-destructive hover:bg-destructive/20 \
                 focus-visible:border-destructive/40 focus-visible:ring-destructive/20 \
                 dark:bg-destructive/20 dark:hover:bg-destructive/30 \
                 dark:focus-visible:ring-destructive/40"
            }
            Self::Outline => {
                "border-border bg-background hover:bg-muted hover:text-foreground \
                 aria-expanded:bg-muted aria-expanded:text-foreground \
                 dark:border-input dark:bg-input/30 dark:hover:bg-input/50"
            }
            Self::Secondary => {
                "bg-secondary text-secondary-foreground \
                 hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)] \
                 aria-expanded:bg-secondary aria-expanded:text-secondary-foreground"
            }
            Self::Ghost => {
                "hover:bg-muted hover:text-foreground aria-expanded:bg-muted \
                 aria-expanded:text-foreground dark:hover:bg-muted/50"
            }
            Self::Link => "text-primary underline-offset-4 hover:underline",
        }
    }
}

/// The size of a [`button`].
///
/// [`Default`] is `ButtonSize::Default`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonSize {
    /// The standard button height.
    #[default]
    Default,
    /// An extra-small button.
    Xs,
    /// A compact button.
    Sm,
    /// A larger button.
    Lg,
    /// A square button sized for a single icon.
    Icon,
    /// An extra-small square icon button.
    IconXs,
    /// A small square icon button.
    IconSm,
    /// A large square icon button.
    IconLg,
}

impl ButtonSize {
    /// The string key for this size, emitted as `data-size`.
    fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Lg => "lg",
            Self::Icon => "icon",
            Self::IconXs => "icon-xs",
            Self::IconSm => "icon-sm",
            Self::IconLg => "icon-lg",
        }
    }

    /// The Tailwind classes for this size.
    fn classes(self) -> &'static str {
        match self {
            Self::Default => {
                "h-8 gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 \
                 has-data-[icon=inline-start]:pl-2"
            }
            Self::Xs => {
                "h-6 gap-1 rounded-[min(var(--radius-md),10px)] px-2 text-xs \
                 in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 \
                 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3"
            }
            Self::Sm => {
                "h-7 gap-1 rounded-[min(var(--radius-md),12px)] px-2.5 text-[0.8rem] \
                 in-data-[slot=button-group]:rounded-lg has-data-[icon=inline-end]:pr-1.5 \
                 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3.5"
            }
            Self::Lg => {
                "h-9 gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 \
                 has-data-[icon=inline-start]:pl-2"
            }
            Self::Icon => "size-8",
            Self::IconXs => {
                "size-6 rounded-[min(var(--radius-md),10px)] \
                 in-data-[slot=button-group]:rounded-lg [&_svg:not([class*='size-'])]:size-3"
            }
            Self::IconSm => {
                "size-7 rounded-[min(var(--radius-md),12px)] in-data-[slot=button-group]:rounded-lg"
            }
            Self::IconLg => "size-9",
        }
    }
}

/// The classes shared by every button, regardless of variant or size.
const BASE: &str = "group/button inline-flex shrink-0 items-center justify-center rounded-lg \
    border border-transparent bg-clip-padding text-sm font-medium whitespace-nowrap transition-all \
    outline-none select-none focus-visible:border-ring focus-visible:ring-3 \
    focus-visible:ring-ring/50 active:not-aria-[haspopup]:translate-y-px \
    disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive \
    aria-invalid:ring-3 aria-invalid:ring-destructive/20 dark:aria-invalid:border-destructive/50 \
    dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 \
    [&_svg:not([class*='size-'])]:size-4";

/// Builds the full class string for a button of the given `variant` and `size`.
///
/// Use it to give button styling to an element that is not a `<button>`, such
/// as a link styled as a button:
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

/// A button component.
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

    view! {
        <button
            data-slot="button"
            data-variant=(variant.name())
            data-size=(size.name())
            class=(class)
            (attrs)
        >
            (child)
        </button>
    }
}
