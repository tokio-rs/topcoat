use toasty::Db;
use topcoat::{
    Result,
    context::{Cx, app_context, memoize},
};

/// A drink on the menu.
#[derive(Debug, toasty::Model)]
pub struct Drink {
    #[key]
    pub slug: String,
    pub name: String,
    pub tasting_notes: String,
    /// The price in dollars, as `f64` so runtime expressions in the browser
    /// can multiply it by a quantity signal.
    pub price: f64,
    pub roast: Roast,
    menu_order: i64,
}

/// The roast profile a drink is brewed from.
#[derive(Debug, Clone, Copy, toasty::Embed)]
pub enum Roast {
    Light,
    Medium,
    Dark,
}

fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

/// Loads the menu, at most once per request.
///
/// `#[memoize]` caches the result for the duration of a request. The layout,
/// the menu page, and the drink page all call this function, but the Toasty
/// query runs only once per request; watch the server log to confirm.
#[memoize]
async fn query_drinks(cx: &Cx) -> toasty::Result<Vec<Drink>> {
    Drink::all()
        .order_by(Drink::fields().menu_order().asc())
        .exec(&mut db(cx))
        .await
}

pub async fn drinks(cx: &Cx) -> Result<&Vec<Drink>> {
    query_drinks(cx)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()).into())
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
