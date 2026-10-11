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

/// Styles for a panel that fades in and out. The closing delay gives the pointer time
/// to reach the panel. Using `display: none` while closed makes the browser recalculate
/// its position when it opens. `@starting-style` starts the fade from transparent.
const PANEL: StaticClass = class!(
    "hidden z-50 w-64 flex-col gap-2 rounded-lg border",
    "border-border bg-popover p-4 text-popover-foreground opacity-0 shadow-sm",
    "[transition:opacity_150ms_ease-out_300ms,display_150ms_allow-discrete_300ms]",
    "group-hover:flex group-hover:opacity-100 starting:group-hover:opacity-0",
    "group-focus-within:flex group-focus-within:opacity-100",
    "starting:group-focus-within:opacity-0",
);

/// The content shown beside a [`hover_card`]'s trigger.
///
/// Use `side` to choose where the card opens and `align` to line it up with the trigger.
/// By default, it opens below the trigger and is centered on it. The browser tries
/// other positions if there is not enough room in the window.
#[component]
pub async fn hover_card_content(
    /// Where the card opens relative to the trigger.
    #[default]
    side: FloatingSide,
    /// Which edge or center of the trigger the card lines up with.
    #[default]
    align: FloatingAlign,
    /// Attributes to add to the card's `<span>`.
    #[default]
    mut attrs: Attributes,
    /// Content shown inside the card.
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
