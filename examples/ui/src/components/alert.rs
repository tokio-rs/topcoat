use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The visual style of an [`alert`].
///
/// The default is `AlertVariant::Neutral`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AlertVariant {
    /// A plain alert, for information.
    #[default]
    Neutral,
    /// An alert in the destructive color, for errors and failures.
    Destructive,
}

impl AlertVariant {
    /// The Tailwind classes for this variant.
    ///
    /// A variant sets the color of the border and the text. The icon and the
    /// [`alert_title`] inherit the text color. The background stays the page
    /// background. The [`alert_description`] sets its own muted color, so the
    /// variant does not change it.
    fn classes(self) -> StaticClass {
        match self {
            Self::Neutral => class!("border-border text-foreground"),
            Self::Destructive => class!("border-destructive/50 text-destructive"),
        }
    }
}

/// The classes shared by every alert, regardless of variant.
///
/// The alert is a grid with two columns: an optional icon, then the title and
/// the description. Without an icon, the first column and its gap have zero
/// width, so the text starts right at the padding.
const BASE: StaticClass = class!(
    "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border \
     bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 \
     [&>svg]:size-4 [&>svg]:translate-y-0.5",
);

/// A box that draws attention to a message on the page.
///
/// `variant` sets the style and defaults to `Neutral`. Child nodes become the
/// alert's content: an optional icon first, then an [`alert_title`] and an
/// [`alert_description`]. The `attrs` (such as `class`) are forwarded to the
/// `<div>`. A `class` among them is appended to the component's classes.
///
/// The alert has no `role="alert"`, because that role makes screen readers
/// announce the element as soon as it appears. For a message that appears
/// while the user is on the page, pass `role="alert"` in `attrs`.
///
/// ```ignore
/// view! {
///     alert(
///         variant: AlertVariant::Destructive,
///         icon(data: iconify_icon!("lucide:triangle-alert"))
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
    child: Child<'_>,
) -> Result<impl View> {
    // `role="alert"` is deliberately absent: it interrupts a screen reader
    // the moment the element appears, which suits a message arriving during
    // the visit, not one rendered with the page. Pass it among the `attrs`
    // where that is what you want.
    Ok(view! {
        <div class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The heading of an [`alert`], usually one line that says what happened.
#[component]
pub async fn alert_title(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!(
                "col-start-2 font-medium tracking-tight",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    })
}

/// Muted text under an [`alert_title`], with the details.
#[component]
pub async fn alert_description(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "col-start-2 text-sm text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
