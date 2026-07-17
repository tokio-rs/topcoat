use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// The visual style of a [`badge`].
///
/// [`Default`] is `BadgeVariant::Primary`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BadgeVariant {
    /// The primary-filled badge for highlighted statuses.
    #[default]
    Primary,
    /// A muted, tinted fill for neutral statuses.
    Secondary,
    /// A hairline-bordered badge on the page background.
    Outline,
    /// A destructive-filled badge for errors and warnings.
    Destructive,
}

impl BadgeVariant {
    /// The Tailwind classes for this variant.
    ///
    /// Each variant sets its own border color rather than inheriting a
    /// transparent one from [`BASE`]: with two border-color classes on the
    /// same element, stylesheet order (not class order) would decide the
    /// winner.
    fn classes(self) -> &'static str {
        match self {
            Self::Primary => "border-transparent bg-primary text-primary-foreground",
            Self::Secondary => "border-transparent bg-foreground/5 text-foreground",
            Self::Outline => "border-border text-foreground",
            Self::Destructive => "border-transparent bg-destructive text-destructive-foreground",
        }
    }
}

/// The classes shared by every badge, regardless of variant.
///
/// Every badge carries a border (colored per variant) so that the `Outline`
/// variant, which only recolors it, does not change the badge's dimensions.
const BASE: &str = "inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md \
    border px-2 py-0.5 text-xs font-medium whitespace-nowrap";

/// Builds the full class string for a badge of the given `variant`.
///
/// Use it to give badge styling to another element, such as a link:
///
/// ```rust
/// view! {
///     <a href="/releases/v2" class=(badge_variants(BadgeVariant::Outline))>"v2.0"</a>
/// }
/// ```
#[must_use]
pub fn badge_variants(variant: BadgeVariant) -> String {
    format!("{BASE} {}", variant.classes())
}

/// A badge component: a small inline pill for statuses, counts, and tags.
///
/// The `variant` parameter selects the styling, defaulting to `Primary`. The
/// `attrs` (such as `class` or `title`) are forwarded to the underlying
/// `<span>`; a `class` among them is appended to the computed classes. Child
/// nodes become the badge's content.
///
/// ```rust
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
    #[default] child: View,
) -> Result {
    view! {
        <span class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </span>
    }
}
