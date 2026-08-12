use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The visual style of an [`alert`].
///
/// [`Default`] is `AlertVariant::Neutral`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AlertVariant {
    /// A plain alert for informational notices.
    #[default]
    Neutral,
    /// A destructive-colored alert for errors and failures.
    Destructive,
}

impl AlertVariant {
    /// The Tailwind classes for this variant.
    ///
    /// A variant colors the border and the text, which the icon and the
    /// [`alert_title`] inherit; the fill stays the page background so the
    /// alert reads as a notice rather than as a banner. The
    /// [`alert_description`] sets its own muted color and so keeps it.
    fn classes(self) -> StaticClass {
        match self {
            Self::Neutral => class!("border-border text-foreground"),
            Self::Destructive => class!("border-destructive/50 text-destructive"),
        }
    }
}

/// The classes shared by every alert, regardless of variant.
///
/// The alert is a two-column grid: an optional leading icon, then the title
/// and the description. Without an icon the first column collapses to nothing
/// and the gap along with it, so the text starts at the padding either way.
const BASE: StaticClass = class!(
    "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border \
     bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 \
     [&>svg]:size-4 [&>svg]:translate-y-0.5",
);

/// An alert component: a notice calling out something about the page it sits
/// on.
///
/// The `variant` parameter selects the styling, defaulting to `Neutral`. Child
/// nodes are the alert's content: an optional icon first, then an
/// [`alert_title`] and an [`alert_description`]. The `attrs` (such as `class`)
/// are forwarded to the underlying `<div>`; a `class` among them is appended
/// to the computed classes.
///
/// ```ignore
/// view! {
///     alert(
///         variant: AlertVariant::Destructive,
///         icon(data: iconify_icon!("feather:alert-triangle"))
///         alert_title("Build failed")
///         alert_description("The last deploy did not finish.")
///     )
/// }
/// ```
#[component]
pub async fn alert(
    /// The visual style of the notice.
    #[default]
    variant: AlertVariant,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The alert's icon, title, and description.
    #[default]
    child: View,
) -> Result {
    // `role="alert"` is deliberately absent: it interrupts a screen reader
    // the moment the element appears, which suits a message arriving during
    // the visit, not one rendered with the page. Pass it among the `attrs`
    // where that is what you want.
    view! {
        <div class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </div>
    }
}

/// The heading of an [`alert`], one line saying what happened.
#[component]
pub async fn alert_title(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <p
            class=(class!(
                "col-start-2 font-medium tracking-tight",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    }
}

/// The supporting text under an [`alert_title`], with the detail and what to
/// do about it.
#[component]
pub async fn alert_description(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!(
                "col-start-2 text-sm text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    }
}
