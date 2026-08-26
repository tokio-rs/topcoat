pub mod drink;

use topcoat::{
    Result,
    context::Cx,
    router::{href, page},
    runtime::{Event, shard},
    view::{View, attributes, component, view},
};

use crate::{
    components::{
        badge::{BadgeVariant, badge, badge_variants},
        button::{ButtonVariant, button},
        card::{card, card_content, card_description, card_header, card_title},
        input::input,
    },
    models::{Drink, Roast, drinks},
};

// The `menu` module adds a URL segment: this page renders at /menu.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        // The signal lives in the browser; typing filters without a reload.
        signal query = String::new();

        <h1 class="text-3xl font-bold tracking-tight">"The menu"</h1>

        <div class="mt-6 flex gap-2">
            input(
                attrs: attributes! {
                    type="search"
                    placeholder="Search the menu..."
                    :value=$(query.get())
                    @input=$(|e: Event| query.set(e.target.value))
                }
            )
            button(
                variant: ButtonVariant::Outline,
                attrs: attributes! {
                    type="button"
                    :disabled=$(query.get().is_empty())
                    @click=$(|_e: Event| query.set("".to_owned()))
                },
                "Clear"
            )
        </div>

        // The shard renders again on the server whenever `query` changes.
        <div class="mt-6">drink_grid(query: $(query.get()))</div>
    })
}

/// The drinks matching the search, rendered on the server.
#[shard]
async fn drink_grid(cx: &Cx, query: String) -> Result<impl View> {
    // The query comes from the client, so treat it as untrusted input.
    let needle = query.trim().to_lowercase();

    let matches: Vec<&Drink> = drinks(cx)
        .await?
        .iter()
        .filter(|drink| drink.name.to_lowercase().contains(&needle))
        .collect();

    Ok(view! {
        if matches.is_empty() {
            <p class="text-muted-foreground">
                "Nothing matches. The barista suggests an espresso."
            </p>
        } else {
            <div class="grid gap-4 sm:grid-cols-2">
                for drink in matches {
                    drink_card(drink: drink)
                }
            </div>
        }
    })
}

/// One menu entry, linking to the drink's page.
#[component]
async fn drink_card(drink: &Drink) -> Result<impl View> {
    Ok(view! {
        <a href=(href!(drink::page, drink::Slug(&drink.slug)))>
            card(
                // The grid stretches every cell to the row height; the card
                // fills it and pins the price to the bottom.
                attrs: attributes! { class="h-full justify-between" },
                card_header(
                    <div class="flex items-center justify-between gap-4">
                        card_title((&drink.name))
                        roast_badge(roast: drink.roast)
                    </div>
                    card_description((&drink.tasting_notes))
                )
                card_content(
                    // Anything can borrow a badge's looks: `badge_variants`
                    // returns the class string for a variant.
                    <p class=(badge_variants(BadgeVariant::Outline))>
                        "$"
                        (drink.price)
                    </p>
                )
            )
        </a>
    })
}

/// The roast as a badge; each profile gets its own weight.
#[component]
async fn roast_badge(roast: Roast) -> Result<impl View> {
    Ok(view! {
        match roast {
            Roast::Light => badge(variant: BadgeVariant::Outline, "Light roast"),
            Roast::Medium => badge(variant: BadgeVariant::Secondary, "Medium roast"),
            Roast::Dark => badge(variant: BadgeVariant::Primary, "Dark roast"),
        }
    })
}
