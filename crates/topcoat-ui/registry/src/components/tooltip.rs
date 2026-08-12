use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// A tooltip component: a hint that shows while its trigger is hovered or
/// focused.
///
/// Child nodes are the trigger and the [`tooltip_content`] holding the hint.
/// The hint shows on hover and on keyboard focus, both of which are the
/// browser's own doing, so nothing here needs scripting. It also never moves
/// out of the way of the viewport's edge, which does: keep hints short, and
/// give the content a side that has room.
///
/// The hint is only a hint. Say what the trigger does in the trigger itself,
/// through its text or an `aria-label`, since a reader who never hovers will
/// not meet the tooltip at all.
///
/// ```ignore
/// view! {
///     tooltip(
///         button(
///             size: ButtonSize::Icon,
///             variant: ButtonVariant::Outline,
///             icon(data: iconify_icon!("feather:copy"), label: "Copy link")
///         )
///         tooltip_content("Copy link")
///     )
/// }
/// ```
#[component]
pub async fn tooltip(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <span
            class=(class!("group relative inline-flex", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    }
}

/// The classes for the [`tooltip_content`] bubble.
///
/// The bubble is painted in the foreground color on the background one, the
/// reverse of the page, which is what sets a hint apart from the surface it
/// covers. It sits above the trigger, centered on it, and takes no pointer
/// events, so hovering along the way to it never gets in the way of what is
/// underneath.
///
/// It fades in on hover and on focus within the trigger, and the visibility
/// that keeps it out of the way in between is named in the transition with
/// `allow-discrete`: it has no in-between values, so without that it would
/// snap and the fade would play against a hint that is already gone. Naming
/// both properties one by one is what makes that work; `all` does not carry
/// the visibility along.
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
pub async fn tooltip_content(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <span role="tooltip" class=(class!(BUBBLE, attrs.remove("class"))) (attrs)>
            (child)
        </span>
    }
}
