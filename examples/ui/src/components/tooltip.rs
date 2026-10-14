use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::floating::{
    FLOATING_SCOPE, FloatingAlign, FloatingAnchor, FloatingSide, floating_variants,
};

/// A short hint shown when its trigger is hovered or focused.
///
/// Add the trigger and a [`tooltip_content`] as children.
///
/// Give the trigger its own text or accessible label. The tooltip must not be the only
/// way to learn what the trigger does. To associate the hint with the trigger, give
/// `tooltip_content` an `id` from [`element_id`](topcoat::view::element_id) and
/// reference it with the trigger's `aria-describedby`.
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

/// Styles for a hint that fades in and out and lets pointer events pass through.
/// Using `display: none` while hidden makes the browser recalculate its position when
/// it appears. `@starting-style` starts the fade from transparent.
const BUBBLE: StaticClass = class!(
    "pointer-events-none hidden z-50 rounded-md bg-foreground px-2.5 py-1",
    "text-xs font-medium text-background",
    "opacity-0 shadow-sm whitespace-nowrap",
    "[transition:opacity_150ms_ease-out,display_150ms_allow-discrete]",
    "group-hover:block group-hover:opacity-100 starting:group-hover:opacity-0",
    "group-focus-within:block group-focus-within:opacity-100",
    "starting:group-focus-within:opacity-0",
);

/// The hint shown beside a [`tooltip`]'s trigger.
///
/// Use `side` to choose where the hint appears and `align` to line it up with the
/// trigger. By default, it appears above the trigger and is centered on it. The browser
/// tries other positions if there is not enough room in the window.
#[component]
pub async fn tooltip_content(
    /// Where the hint appears relative to the trigger.
    #[default(FloatingSide::Top)]
    side: FloatingSide,
    /// Which edge or center of the trigger the hint lines up with.
    #[default]
    align: FloatingAlign,
    /// Attributes to add to the hint's `<span>`.
    #[default]
    mut attrs: Attributes,
    /// Content shown in the hint.
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
