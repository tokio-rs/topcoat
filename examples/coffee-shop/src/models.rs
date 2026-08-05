mod drink;

pub use drink::*;
use toasty::Db;
use topcoat::context::{Cx, app_context};

pub(crate) fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

pub async fn seed(db: &mut Db) -> toasty::Result<()> {
    toasty::create!(Drink::[
        {
            slug: "espresso",
            name: "Espresso",
            tasting_notes: "A quick, syrupy shot under a hazelnut crema.",
            price: 3.0,
            roast: Roast::Dark,
            menu_order: 0,
        },
        {
            slug: "cold-brew",
            name: "Cold Brew",
            tasting_notes: "Steeped overnight: chocolate, cherry, no acidity.",
            price: 4.0,
            roast: Roast::Dark,
            menu_order: 1,
        },
        {
            slug: "cappuccino",
            name: "Cappuccino",
            tasting_notes: "Equal parts espresso, steamed milk, and foam.",
            price: 5.0,
            roast: Roast::Medium,
            menu_order: 2,
        },
        {
            slug: "flat-white",
            name: "Flat White",
            tasting_notes: "Two ristretto shots under velvet microfoam.",
            price: 5.0,
            roast: Roast::Medium,
            menu_order: 3,
        },
        {
            slug: "pour-over",
            name: "Pour Over",
            tasting_notes: "A single-origin Ethiopian, bright and floral.",
            price: 6.0,
            roast: Roast::Light,
            menu_order: 4,
        },
        {
            slug: "mocha",
            name: "Mocha",
            tasting_notes: "Espresso and dark chocolate under whipped cream.",
            price: 6.0,
            roast: Roast::Dark,
            menu_order: 5,
        },
    ])
    .exec(db)
    .await?;

    Ok(())
}
