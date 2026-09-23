use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`card`] container.
///
/// The card is a column of sections with an even gap between them. The card
/// only has vertical padding. Each section has its own horizontal padding, so
/// content such as an image can span the full width of the card. The card has
/// a shadow and sets its own background and text color, so it looks the same
/// on any background.
const CARD: StaticClass = class!(
    "flex flex-col gap-5 rounded-xl border border-border bg-card py-6 \
     text-card-foreground shadow-sm",
);

/// A raised surface with a border that groups related content.
///
/// A card stacks its sections vertically. Usually these are a
/// [`card_header`], a [`card_content`], and a [`card_footer`], in that order.
/// Each section is optional. Child nodes become the card's sections.
///
/// The `attrs` (such as `class` or event handlers) are forwarded to the
/// `<div>`. A `class` among them is appended to the component's classes. The
/// same holds for the section components.
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

/// The first section of a [`card`]. It holds a [`card_title`] and an
/// optional [`card_description`], stacked vertically.
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

/// Muted text under a [`card_title`], rendered as a `<p>`.
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

/// The last section of a [`card`]: a horizontal row, usually for actions.
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
