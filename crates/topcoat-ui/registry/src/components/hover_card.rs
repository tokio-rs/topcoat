use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A card with details about its trigger, shown while the trigger is hovered
/// or focused.
///
/// A tooltip shows a few words, and a hover card shows a whole view, such as
/// the profile behind a mention or a preview behind a link. Child nodes are
/// the trigger and a [`hover_card_content`] with the view. It works without
/// scripting.
///
/// The card appears after a short delay, so it does not show when the cursor
/// only passes over the trigger. It also waits before it hides, so the cursor
/// can move onto the card. Users who do not hover, such as users of touch
/// screens, never see the card, so do not put anything in it that is not
/// available elsewhere.
///
/// The `attrs` are forwarded to the wrapping `<span>`. A `class` among them
/// is appended to the component's classes. The same holds for
/// [`hover_card_content`].
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
            class=(class!("group relative inline-flex", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// The classes for the [`hover_card_content`] panel.
///
/// The panel is a raised surface like a card, below the trigger and aligned
/// to its left edge. It sets its own background and text color, so it looks
/// the same over any content. Unlike a tooltip, it receives pointer events,
/// so it can contain links.
///
/// The delay before the fade in and out keeps it from flickering when the
/// cursor passes by. The transition lists `visibility` with
/// `allow-discrete`, so the panel stays visible until the fade out ends. Both
/// properties are listed by name, because `all` does not include
/// `visibility`.
const PANEL: StaticClass = class!(
    "invisible absolute top-full left-0 z-50 mt-2 w-64 rounded-lg border \
     border-border bg-popover p-4 text-popover-foreground opacity-0 shadow-sm \
     [transition:opacity_150ms_ease-out_300ms,visibility_150ms_allow-discrete_300ms] \
     group-hover:visible group-hover:opacity-100 \
     group-focus-within:visible group-focus-within:opacity-100",
);

/// The content of a [`hover_card`], shown in a panel below the trigger.
#[component]
pub async fn hover_card_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!("flex flex-col gap-2", PANEL, attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}
