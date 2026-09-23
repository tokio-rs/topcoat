use topcoat::{
    Result,
    view::{Attributes, Child, Class, StaticClass, View, class, component, view},
};

/// The visual style of a [`button`].
///
/// The default is `ButtonVariant::Primary`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    /// Filled with the primary color, for the main action.
    #[default]
    Primary,
    /// A muted tinted fill, for secondary actions.
    Secondary,
    /// A thin border and no fill.
    Outline,
    /// No fill until hovered, for toolbars and inline actions.
    Ghost,
    /// Filled with the destructive color, for actions such as deleting data.
    Destructive,
}

impl ButtonVariant {
    /// The Tailwind classes for this variant.
    ///
    /// Hover and press states use the fill or foreground color at a lower
    /// opacity, so they work in both color schemes without `dark:` classes.
    /// Variants with a fill have a small shadow. Outline and ghost buttons
    /// have none.
    ///
    /// Each variant sets its own border color instead of getting a
    /// transparent one from [`BASE`]. With two border color classes on one
    /// element, the order in the stylesheet would decide which one wins, not
    /// the order of the classes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Primary => class!(
                "border-transparent bg-primary text-primary-foreground shadow-xs \
                 hover:bg-primary/90 active:bg-primary/80",
            ),
            Self::Secondary => class!(
                "border-transparent bg-foreground/5 text-foreground shadow-xs \
                 hover:bg-foreground/10 active:bg-foreground/15",
            ),
            Self::Outline => class!(
                "border-border text-foreground hover:bg-foreground/5 \
                 active:bg-foreground/10",
            ),
            Self::Ghost => class!(
                "border-transparent text-foreground hover:bg-foreground/5 active:bg-foreground/10",
            ),
            Self::Destructive => class!(
                "border-transparent bg-destructive text-destructive-foreground shadow-xs \
                 hover:bg-destructive/90 active:bg-destructive/80",
            ),
        }
    }
}

/// The size of a [`button`].
///
/// The default is `ButtonSize::Md`.
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
    /// A square button for a single icon.
    Icon,
}

impl ButtonSize {
    /// The Tailwind classes for this size.
    ///
    /// Sizes change the dimensions of the button. The text size stays the
    /// same.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-8 gap-1.5 rounded-md px-3"),
            Self::Md => class!("h-9 gap-2 rounded-lg px-4"),
            Self::Lg => class!("h-10 gap-2 rounded-lg px-5"),
            Self::Icon => class!("size-9 rounded-lg"),
        }
    }
}

/// The classes shared by every button, regardless of variant or size.
///
/// Every button has a border, colored by its variant. So the `Outline`
/// variant, which only shows the border, has the same size as the others.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 items-center justify-center border \
     text-sm font-medium whitespace-nowrap transition-colors outline-none select-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
);

/// Returns the full class list of a button with the given `variant` and
/// `size`.
///
/// Use it to style an element that is not a `<button>`, such as a link, like
/// a button:
///
/// ```ignore
/// view! {
///     <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
///         "Sign in"
///     </a>
/// }
/// ```
#[must_use]
pub fn button_variants(
    variant: ButtonVariant,
    size: ButtonSize,
) -> Class<(StaticClass, StaticClass, StaticClass)> {
    class!(BASE, variant.classes(), size.classes())
}

/// A button.
///
/// `variant` and `size` set the style and default to `Primary` and `Md`. The
/// `attrs` (such as `class`, `type`, `disabled`, or event handlers) are
/// forwarded to the `<button>`. A `class` among them is appended to the
/// component's classes. Child nodes become the button's content.
///
/// ```ignore
/// view! {
///     button(
///         variant: ButtonVariant::Destructive,
///         attrs: attributes! { type="submit" },
///         "Delete"
///     )
/// }
/// ```
///
/// To style another element like a button, use [`button_variants`].
#[component]
pub async fn button(
    #[default] variant: ButtonVariant,
    #[default] size: ButtonSize,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            class=(class!(
                BASE,
                variant.classes(),
                size.classes(),
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </button>
    })
}
