use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
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
    /// Classes for the variant's border and text colors.
    fn classes(self) -> StaticClass {
        match self {
            Self::Neutral => class!("border-border text-foreground"),
            Self::Destructive => class!("border-destructive/50 text-destructive"),
        }
    }
}

/// Classes for the alert layout. The icon column collapses when no icon is present.
const BASE: StaticClass = class!(
    "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border \
     bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 \
     [&>svg]:size-4 [&>svg]:translate-y-0.5",
);

/// A notice displayed within the page.
///
/// Use `variant` to choose its style. Pass an optional icon, an `alert_title`, and an
/// `alert_description` as children. `attrs` are forwarded to the `<div>`, with extra
/// classes added to its classes.
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

/// The heading of an alert.
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

/// Text that explains the alert and any action the reader should take.
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
