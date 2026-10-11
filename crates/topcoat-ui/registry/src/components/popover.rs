use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::floating::{FloatingAlign, FloatingAnchor, FloatingSide, floating_variants};

/// A panel that opens beside a button.
///
/// Add a [`popover_trigger`] and a [`popover_content`] as children. Use
/// [`element_id`](topcoat::view::element_id) to create an ID, then pass it to the
/// trigger's `target` and the content's `id`. The button opens and closes the panel
/// without JavaScript. Clicking outside or pressing Escape also closes it. The panel
/// appears above the page and is not cut off by a parent's overflow settings.
///
/// `attrs` adds attributes and classes to the wrapper `<div>`. The wrapper takes up no
/// space in the layout. Use `group-has-[:popover-open]/popover:` classes to change how
/// children look while the panel is open.
///
/// ```ignore
/// view! {
///     let share = element_id(cx);
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

/// A button for opening and closing [`popover_content`].
///
/// Set `target` to the panel's `id` and put the button's label in its children.
/// The button has no visual style by default. To give it the standard button style,
/// add [`button_variants`](super::button::button_variants) classes through `attrs`.
/// All attributes in `attrs` go on the `<button>`.
#[component]
pub async fn popover_trigger(
    /// The panel ID this button controls.
    target: &str,
    /// Attributes to add to the button.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the button.
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

/// The panel's size, colors, border, spacing, and shadow.
const PANEL: StaticClass = class!(
    "w-72 rounded-lg border border-border bg-popover p-4 text-popover-foreground",
    "shadow-sm outline-none",
);

/// The content shown when a [`popover`] opens.
///
/// Use `side` to choose where the panel opens and `align` to line it up with the
/// button. By default, it opens below the button and is centered on it. The browser
/// tries other positions if there is not enough room in the window.
///
/// `attrs` adds attributes and classes to the panel's `<div>`. Add width classes here
/// to change the panel's size.
#[component]
pub async fn popover_content(
    /// The panel ID to pass to the trigger's `target`.
    id: &str,
    /// Where the panel opens relative to the button.
    #[default]
    side: FloatingSide,
    /// Which edge or center of the button the panel lines up with.
    #[default]
    align: FloatingAlign,
    /// Attributes to add to the panel's `<div>`.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the panel.
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
