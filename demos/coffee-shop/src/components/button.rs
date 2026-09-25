use topcoat::{
    Result,
    view::{Attributes, Child, Class, StaticClass, View, class, component, view},
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
    /// Classes for the button variant and its interaction states.
    ///
    /// Each variant sets its own border color. Keep border colors out of the shared
    /// base to avoid conflicting classes.
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
    /// Classes for the button dimensions. Text size stays the same across sizes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-8 gap-1.5 rounded-md px-3"),
            Self::Md => class!("h-9 gap-2 rounded-lg px-4"),
            Self::Lg => class!("h-10 gap-2 rounded-lg px-5"),
            Self::Icon => class!("size-9 rounded-lg"),
        }
    }
}

/// Classes shared by button variants and sizes. A border reserves the same space in
/// every variant.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 items-center justify-center border \
     text-sm font-medium whitespace-nowrap transition-colors outline-none select-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50",
);

/// Builds the full class list for a button of the given `variant` and `size`.
///
/// Use it to give button styling to an element that is not a `<button>`, such
/// as a link styled as a button:
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

/// A styled button.
///
/// `variant` defaults to `Primary` and `size` to `Md`. Pass the content as children.
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
/// Use [`button_variants`] to apply the same styling to another element.
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
