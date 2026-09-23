use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A short hint shown when its trigger is hovered or focused.
///
/// Provide a trigger followed by `tooltip_content`. It uses CSS and does not
/// reposition itself near viewport edges, so keep the text short and allow
/// space around the trigger.
///
/// Give the trigger its own text or accessible label. The tooltip must not be
/// the only way to understand the control.
///
/// ```ignore
/// view! {
///     tooltip(
///         button(
///             size: ButtonSize::Icon,
///             variant: ButtonVariant::Outline,
///             icon(data: iconify_icon!("lucide:copy"), label: "Copy link")
///         )
///         tooltip_content("Copy link")
///     )
/// }
/// ```
#[component]
pub async fn tooltip(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!("group relative inline-flex", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// The classes for the [`tooltip_content`] bubble.
const BUBBLE: StaticClass = class!(
    "pointer-events-none invisible absolute bottom-full left-1/2 z-50 mb-2 \
     -translate-x-1/2 rounded-md bg-foreground px-2.5 py-1 text-xs font-medium text-background \
     opacity-0 shadow-sm whitespace-nowrap \
     [transition:opacity_150ms_ease-out,visibility_150ms_allow-discrete] \
     group-hover:visible group-hover:opacity-100 \
     group-focus-within:visible group-focus-within:opacity-100",
);

/// The hint a [`tooltip`] shows, in a bubble above its trigger.
#[component]
pub async fn tooltip_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span role="tooltip" class=(class!(BUBBLE, attrs.remove("class"))) (attrs)>
            (child)
        </span>
    })
}
