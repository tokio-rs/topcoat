use topcoat::{
    Result,
    context::Cx,
    router::{error::RouterErrorExt, page, path_param},
    runtime::{Event, procedure},
    view::{attributes, view},
};

use super::roast_badge;
use crate::{
    components::button::{ButtonSize, ButtonVariant, button},
    customer::current_customer,
    models::{Drink, db},
};

// The declaration turns this module's segment into a parameter, so the page
// below renders at /menu/{slug}.
path_param!(slug);

#[page]
async fn drink_page(cx: &Cx) -> Result {
    let slug = path_param::<Slug>(cx);

    let drink = Drink::filter_by_slug(slug)
        .first()
        .exec(&mut db(cx))
        .await?
        .ok_or_not_found()?;

    // Snapshots captured by the runtime expressions below.
    let name = drink.name.clone();
    let price = drink.price;

    view! {
        signal quantity = 1.0;
        signal confirmation = String::new();

        <a href="/menu" class="text-sm text-muted-foreground hover:text-foreground">
            "Back to the menu"
        </a>

        <div class="mt-4 flex items-center gap-4">
            <h1 class="text-3xl font-bold tracking-tight">(&drink.name)</h1>
            roast_badge(roast: drink.roast)
        </div>

        <p class="mt-3 max-w-md text-muted-foreground">(&drink.tasting_notes)</p>

        <div class="mt-8 flex items-center gap-3">
            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Icon,
                attrs: attributes! {
                    @click=$(|_e: Event| {
                        if 1.0 < quantity.get() {
                            quantity.decrement()
                        } else {
                            quantity.set(1.0)
                        }
                    })
                },

                "-"
            )
            <p class="w-8 text-center text-lg font-medium">$(quantity.get())</p>

            button(
                variant: ButtonVariant::Outline,
                size: ButtonSize::Icon,
                attrs: attributes! { @click=$(|_e: Event| quantity.increment()) },
                "+"
            )

            // Re-renders whenever the quantity changes; `price` was captured
            // during the server render.
            <p class="ml-2 text-lg">
                "$"
                $(quantity.get() * price)
            </p>
        </div>

        <div class="mt-6 flex items-center gap-4">
            // The click handler calls the procedure on the server and puts
            // its return value into the `confirmation` signal.
            button(
                attrs: attributes! {
                    @click=$(async move |_e: Event| {
                        let message = place_order(name.to_owned(), quantity.get()).await;
                        confirmation.set(message);
                    })
                },
                "Order"
            )

            <p :hidden=$(confirmation.get().is_empty()) class="text-sm">
                $(confirmation.get())
            </p>
        </div>
    }
}

// The arguments come from the client, so a real application would validate
// them before ringing anything up.
#[procedure]
async fn place_order(cx: &Cx, drink: String, quantity: f64) -> Result<String> {
    let greeting = match current_customer(cx) {
        Some(name) => format!("Coming right up, {name}"),
        None => "Coming right up".to_owned(),
    };

    Ok(format!("{greeting}: {quantity} x {drink}."))
}
