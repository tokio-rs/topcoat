use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
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
            class=(class!("group relative inline-flex", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// Classes for the panel below the trigger. Delayed opacity and visibility transitions
/// let the pointer reach the panel before it closes.
const PANEL: StaticClass = class!(
    "invisible absolute top-full left-0 z-50 mt-2 w-64 rounded-lg border \
     border-border bg-popover p-4 text-popover-foreground opacity-0 shadow-sm \
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
