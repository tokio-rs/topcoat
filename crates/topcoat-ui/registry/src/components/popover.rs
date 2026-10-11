use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::floating::{FloatingAlign, FloatingAnchor, FloatingSide, floating_variants};

/// A panel of rich content that opens next to a trigger.
///
/// Pass a [`popover_trigger`] and a [`popover_content`] as children, and connect them by
/// passing the content's `id` to the trigger's `target`. Create the id with
/// [`element_id`](topcoat::view::element_id). The browser opens and closes the panel
/// without JavaScript. Clicking outside the panel or pressing Escape closes it. The panel
/// shows above the rest of the page, so containers that clip their overflow do not cut
/// it off.
///
/// `attrs` are forwarded to the wrapper `<div>`, with extra classes added to its classes.
/// The wrapper does not generate a box of its own. Use `group-has-[:popover-open]/popover:`
/// classes to style children while the panel is open.
///
/// ```ignore
/// let share = element_id(cx);
/// view! {
///     popover(
///         popover_trigger(
///             target: &share,
///             attrs: attributes! {
///                 class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))
///             },
///             "Share"
///         )
///         popover_content(id: &share, align: FloatingAlign::End, (share_form))
///     )
/// }
/// ```
#[component]
pub async fn popover(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("group/popover contents", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The button that opens or closes a [`popover_content`].
///
/// Set `target` to the content's `id`. Pass the label as children. The trigger has no
/// styling of its own. To style it as a button, pass classes from
/// [`button_variants`](super::button::button_variants) through `attrs`. The attributes
/// go on the `<button>`.
#[component]
pub async fn popover_trigger(
    /// The `id` of the panel to open.
    target: &str,
    /// Extra attributes for the `<button>` element.
    #[default]
    mut attrs: Attributes,
    /// The trigger's label.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            type="button"
            popovertarget=(target)
            class=(class!("cursor-pointer", attrs.remove("class")))
            (attrs)
        >
            (child)
        </button>
    })
}

/// Classes for a popover panel with its own background, border, and text color.
const PANEL: StaticClass = class!(
    "w-72 rounded-lg border border-border bg-popover p-4 text-popover-foreground",
    "shadow-sm outline-none",
);

/// The floating panel of a [`popover`].
///
/// `side` chooses the side of the trigger it opens on and defaults to `Bottom`. `align`
/// lines it up with the trigger and defaults to `Center`. When the panel would overflow
/// the viewport, it moves to another position.
///
/// `attrs` are forwarded to the `<div>`, with extra classes added to its classes. Use
/// them to change the panel's width.
#[component]
pub async fn popover_content(
    /// The panel's `id`, referenced by its trigger's `target`.
    id: &str,
    /// The side of the trigger the panel opens on.
    #[default]
    side: FloatingSide,
    /// How the panel lines up with the trigger.
    #[default]
    align: FloatingAlign,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The panel's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            id=(id)
            popover="auto"
            class=(class!(
                floating_variants(FloatingAnchor::Invoker, side, align),
                PANEL,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
