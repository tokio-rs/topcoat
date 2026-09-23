use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A short hint that shows while its trigger is hovered or focused.
///
/// Child nodes are the trigger and a [`tooltip_content`] with the hint. The
/// hint shows on hover and on keyboard focus without any scripting. It always
/// appears above the trigger and does not move away from the edge of the
/// viewport, so keep hints short and leave room above the trigger.
///
/// Users who do not hover never see the tooltip. So the trigger itself should
/// say what it does, through its text or an `aria-label`.
///
/// The `attrs` are forwarded to the wrapping `<span>`. A `class` among them
/// is appended to the component's classes. The same holds for
/// [`tooltip_content`].
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
///
/// The bubble swaps the page colors: the background is the foreground color
/// and the text is the background color. This sets the hint apart from the
/// page. It sits above the trigger, centered, and ignores pointer events, so
/// it never blocks the content under it.
///
/// It fades in on hover and on focus. The transition lists `visibility` with
/// `allow-discrete`, so the bubble stays visible until the fade out ends.
/// Both properties are listed by name, because `all` does not include
/// `visibility`.
const BUBBLE: StaticClass = class!(
    "pointer-events-none invisible absolute bottom-full left-1/2 z-50 mb-2 \
     -translate-x-1/2 rounded-md bg-foreground px-2.5 py-1 text-xs font-medium text-background \
     opacity-0 shadow-sm whitespace-nowrap \
     [transition:opacity_150ms_ease-out,visibility_150ms_allow-discrete] \
     group-hover:visible group-hover:opacity-100 \
     group-focus-within:visible group-focus-within:opacity-100",
);

/// The hint of a [`tooltip`], shown in a bubble above the trigger.
///
/// It has `role="tooltip"`.
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
