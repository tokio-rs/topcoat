use topcoat::{
    Result,
    view::{Attributes, Child, Class, StaticClass, View, class, component, view},
};

/// The visual style of a [`badge`].
///
/// The default is `BadgeVariant::Primary`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BadgeVariant {
    /// Filled with the primary color, for highlighted statuses.
    #[default]
    Primary,
    /// A muted tinted fill, for neutral statuses.
    Secondary,
    /// A thin border and no fill.
    Outline,
    /// Filled with the destructive color, for errors and warnings.
    Destructive,
}

impl BadgeVariant {
    /// The Tailwind classes for this variant.
    ///
    /// Each variant sets its own border color instead of getting a
    /// transparent one from [`BASE`]. With two border color classes on one
    /// element, the order in the stylesheet would decide which one wins, not
    /// the order of the classes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Primary => class!("border-transparent bg-primary text-primary-foreground"),
            Self::Secondary => class!("border-transparent bg-foreground/5 text-foreground"),
            Self::Outline => class!("border-border text-foreground"),
            Self::Destructive => {
                class!("border-transparent bg-destructive text-destructive-foreground")
            }
        }
    }
}

/// The classes shared by every badge, regardless of variant.
///
/// Every badge has a border, colored by its variant. So the `Outline`
/// variant, which only shows the border, has the same size as the others.
const BASE: StaticClass = class!(
    "inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md \
     border px-2 py-0.5 text-xs font-medium whitespace-nowrap",
);

/// Returns the full class list of a badge with the given `variant`.
///
/// Use it to style another element, such as a link, like a badge:
///
/// ```ignore
/// view! {
///     <a href="/releases/v2" class=(badge_variants(BadgeVariant::Outline))>"v2.0"</a>
/// }
/// ```
#[must_use]
pub fn badge_variants(variant: BadgeVariant) -> Class<(StaticClass, StaticClass)> {
    class!(BASE, variant.classes())
}

/// A small inline label for statuses, counts, and tags.
///
/// `variant` sets the style and defaults to `Primary`. The `attrs` (such as
/// `class` or `title`) are forwarded to the `<span>`. A `class` among them is
/// appended to the component's classes. Child nodes become the badge's
/// content.
///
/// ```ignore
/// view! {
///     badge(variant: BadgeVariant::Destructive, "Failed")
/// }
/// ```
///
/// To style another element like a badge, use [`badge_variants`] directly.
#[component]
pub async fn badge(
    #[default] variant: BadgeVariant,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </span>
    })
}
