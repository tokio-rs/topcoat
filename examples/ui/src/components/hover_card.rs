use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A hover card component: a card of detail about its trigger, shown while
/// the trigger is hovered or focused.
///
/// Where a tooltip carries a few words, a hover card carries a view: the
/// person behind a mention, the project behind a link. Child nodes are the
/// trigger and the [`hover_card_content`] holding that view.
///
/// The card waits before it appears, so that passing the cursor over the
/// trigger on the way somewhere else does not summon it, and waits again
/// before it goes, so that the way to it is not a race. What is in it is
/// detail, not the only copy of anything: a reader who never hovers, and one
/// on a touch screen, will not see it.
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
/// The panel is a raised surface like the card's, dropped below the trigger
/// and aligned to its left edge. It sets its own background and text color,
/// so it reads the same over anything it covers, and it takes pointer events,
/// unlike a tooltip's bubble: a hover card can hold a link worth following.
///
/// The delay before it fades in and out is what keeps it from flickering as
/// the cursor passes by. The visibility that keeps it out of the way in
/// between is named in the transition with `allow-discrete`, since it has no
/// in-between values; naming both properties one by one is what makes that
/// work, as `all` does not carry the visibility along.
const PANEL: StaticClass = class!(
    "invisible absolute top-full left-0 z-50 mt-2 w-64 rounded-lg border \
     border-border bg-background p-4 text-foreground opacity-0 shadow-sm \
     [transition:opacity_150ms_ease-out_300ms,visibility_150ms_allow-discrete_300ms] \
     group-hover:visible group-hover:opacity-100 \
     group-focus-within:visible group-focus-within:opacity-100",
);

/// The view a [`hover_card`] shows, in a panel below its trigger.
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
