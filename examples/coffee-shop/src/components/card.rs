use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// The classes for the [`card`] container.
///
/// The card is a column of sections separated by a uniform gap. It carries
/// vertical padding only; each section brings its own horizontal padding, so
/// full-bleed content such as an image can span the card's width. The card
/// casts the theme's raised-surface shadow and sets its own background and
/// text color, so it reads as a card on any ancestor.
const CARD: &str = "flex flex-col gap-5 rounded-xl border border-border bg-background py-6 \
    text-foreground shadow-sm";

/// A card component: a bordered, raised surface grouping related content.
///
/// A card stacks sections vertically: typically a [`card_header`], then a
/// [`card_content`], closed by a [`card_footer`]. Any section can be omitted.
/// The `attrs` (such as `class` or event handlers) are forwarded to the
/// underlying `<div>`; a `class` among them is appended to the computed
/// classes. Child nodes become the card's sections.
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
pub async fn card(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <div class=(class!(CARD, attrs.remove("class"))) (attrs)>(child)</div> }
}

/// The opening section of a [`card`], stacking a [`card_title`] and an
/// optional [`card_description`].
#[component]
pub async fn card_header(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!("flex flex-col gap-1.5 px-6", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    }
}

/// The heading of a [`card`], rendered as an `<h3>`.
#[component]
pub async fn card_title(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <h3 class=(class!("leading-none font-semibold", attrs.remove("class"))) (attrs)>
            (child)
        </h3>
    }
}

/// The supporting text under a [`card_title`].
#[component]
pub async fn card_description(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <p
            class=(class!("text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </p>
    }
}

/// The main body of a [`card`].
#[component]
pub async fn card_content(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <div class=(class!("px-6", attrs.remove("class"))) (attrs)>(child)</div> }
}

/// The closing section of a [`card`], a horizontal row for actions.
#[component]
pub async fn card_footer(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!("flex items-center gap-2 px-6", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    }
}
