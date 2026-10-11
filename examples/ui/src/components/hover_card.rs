use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

use super::floating::{
    FLOATING_SCOPE, FloatingAlign, FloatingAnchor, FloatingSide, floating_variants,
};

/// Extra information shown when its trigger is hovered or focused.
///
/// Pass the trigger and a `hover_card_content` panel as children. The panel appears and
/// disappears after a short delay to avoid flickering as the pointer moves. Keep
/// essential information available elsewhere, since touch users may not see the card.
///
/// ```ignore
/// view! {
///     hover_card(
///         <a href="/people/ada" class="font-medium underline">"@ada"</a>
///         hover_card_content(
///             <p class="text-sm font-medium">"Ada Lovelace"</p>
///             <p class="text-sm text-muted-foreground">"Owner, joined in 2024."</p>
///         )
///     )
/// }
/// ```
#[component]
pub async fn hover_card(
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

/// Classes for the panel. The closed panel generates no box, so it is placed again each
/// time it opens. Delayed opacity and display transitions let the pointer reach the
/// panel before it closes, and `@starting-style` supplies the entry opacity.
const PANEL: StaticClass = class!(
    "hidden z-50 w-64 flex-col gap-2 rounded-lg border",
    "border-border bg-popover p-4 text-popover-foreground opacity-0 shadow-sm",
    "[transition:opacity_150ms_ease-out_300ms,display_150ms_allow-discrete_300ms]",
    "group-hover:flex group-hover:opacity-100 starting:group-hover:opacity-0",
    "group-focus-within:flex group-focus-within:opacity-100",
    "starting:group-focus-within:opacity-0",
);

/// The view a [`hover_card`] shows, in a panel next to its trigger.
///
/// `side` chooses the side of the trigger it opens on and defaults to `Bottom`. `align`
/// lines it up with the trigger and defaults to `Center`. When the panel would overflow
/// the viewport, it moves to another position.
#[component]
pub async fn hover_card_content(
    /// The side of the trigger the panel opens on.
    #[default]
    side: FloatingSide,
    /// How the panel lines up with the trigger.
    #[default]
    align: FloatingAlign,
    /// Extra attributes for the `<span>` element.
    #[default]
    mut attrs: Attributes,
    /// The panel's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                floating_variants(FloatingAnchor::Scope, side, align),
                PANEL,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </span>
    })
}
