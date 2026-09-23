use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes for a card with vertically stacked sections. Each section supplies its own
/// horizontal padding so other content can span the full width.
const CARD: StaticClass = class!(
    "flex flex-col gap-5 rounded-xl border border-border bg-card py-6 \
     text-card-foreground shadow-sm",
);

/// A bordered panel that groups related content.
///
/// Pass sections as children. Use a header, body, or footer as needed. `attrs` are
/// forwarded to the `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     card(
///         attrs: attributes! { class="max-w-sm" },
///         card_header(
///             card_title("Delete workspace")
///             card_description("This cannot be undone.")
///         )
///         card_footer(
///             attrs: attributes! { class="justify-end" },
///             button(variant: ButtonVariant::Destructive, "Delete")
///         )
///     )
/// }
/// ```
#[component]
pub async fn card(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <div class=(class!(CARD, attrs.remove("class"))) (attrs)>(child)</div> })
}

/// The opening section of a [`card`], stacking a [`card_title`] and an
/// optional [`card_description`].
#[component]
pub async fn card_header(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("flex flex-col gap-1.5 px-6", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The heading of a [`card`], rendered as an `<h3>`.
#[component]
pub async fn card_title(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <h3
            class=(class!("text-base leading-none font-semibold", attrs.remove("class")))
            (attrs)
        >
            (child)
        </h3>
    })
}

/// The supporting text under a [`card_title`].
#[component]
pub async fn card_description(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!("text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </p>
    })
}

/// The main body of a [`card`].
#[component]
pub async fn card_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <div class=(class!("px-6", attrs.remove("class"))) (attrs)>(child)</div> })
}

/// The closing section of a [`card`], a horizontal row for actions.
#[component]
pub async fn card_footer(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("flex items-center gap-2 px-6", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}
