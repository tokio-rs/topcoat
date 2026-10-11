use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::floating::{
    FLOATING_SCOPE, FloatingAlign, FloatingAnchor, FloatingSide, floating_variants,
};

/// A short hint shown when its trigger is hovered or focused.
///
/// Pass the trigger and `tooltip_content` as children.
///
/// Give the trigger its own text or accessible label. The tooltip must not be the only
/// way to learn what the trigger does. To associate the hint with the trigger, give
/// `tooltip_content` an `id` and reference it with the trigger's `aria-describedby`.
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
            class=(class!("group inline-flex", FLOATING_SCOPE, attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// Classes for a tooltip bubble. The bubble ignores pointer events. Opacity and
/// visibility transitions let it fade in and out.
const BUBBLE: StaticClass = class!(
    "pointer-events-none invisible z-50 rounded-md bg-foreground px-2.5 py-1",
    "text-xs font-medium text-background",
    "opacity-0 shadow-sm whitespace-nowrap",
    "[transition:opacity_150ms_ease-out,visibility_150ms_allow-discrete]",
    "group-hover:visible group-hover:opacity-100",
    "group-focus-within:visible group-focus-within:opacity-100",
);

/// The hint a [`tooltip`] shows, in a bubble next to its trigger.
///
/// `side` chooses the side of the trigger it shows on and defaults to `Top`. `align`
/// lines it up with the trigger and defaults to `Center`. When the bubble would overflow
/// the viewport, it moves to another position.
#[component]
pub async fn tooltip_content(
    /// The side of the trigger the bubble shows on.
    #[default(FloatingSide::Top)]
    side: FloatingSide,
    /// How the bubble lines up with the trigger.
    #[default]
    align: FloatingAlign,
    /// Extra attributes for the `<span>` element.
    #[default]
    mut attrs: Attributes,
    /// The hint.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            role="tooltip"
            class=(class!(
                floating_variants(FloatingAnchor::Scope, side, align),
                BUBBLE,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}
