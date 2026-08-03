use std::time::Duration;

use topcoat::context::memoize;

/// A drink on the menu.
pub struct Drink {
    pub slug: &'static str,
    pub name: &'static str,
    pub tasting_notes: &'static str,
    /// The price in dollars, as `f64` so runtime expressions in the browser
    /// can multiply it by a quantity signal.
    pub price: f64,
    pub roast: Roast,
}

/// The roast profile a drink is brewed from.
#[derive(Clone, Copy)]
pub enum Roast {
    Light,
    Medium,
    Dark,
}

/// Loads the menu, at most once per request.
///
/// `#[memoize]` caches the result for the duration of a request. The layout,
/// the menu page, and the drink page all call this function, but the
/// simulated database query below runs only once per request; watch the
/// server log to confirm.
#[memoize]
pub async fn drinks(cx: &Cx) -> Vec<Drink> {
    // Stand-in for a database query.
    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("loading the menu");

    vec![
        Drink {
            slug: "espresso",
            name: "Espresso",
            tasting_notes: "A quick, syrupy shot under a hazelnut crema.",
            price: 3.0,
            roast: Roast::Dark,
        },
        Drink {
            slug: "cold-brew",
            name: "Cold Brew",
            tasting_notes: "Steeped overnight: chocolate, cherry, no acidity.",
            price: 4.0,
            roast: Roast::Dark,
        },
        Drink {
            slug: "cappuccino",
            name: "Cappuccino",
            tasting_notes: "Equal parts espresso, steamed milk, and foam.",
            price: 5.0,
            roast: Roast::Medium,
        },
        Drink {
            slug: "flat-white",
            name: "Flat White",
            tasting_notes: "Two ristretto shots under velvet microfoam.",
            price: 5.0,
            roast: Roast::Medium,
        },
        Drink {
            slug: "pour-over",
            name: "Pour Over",
            tasting_notes: "A single-origin Ethiopian, bright and floral.",
            price: 6.0,
            roast: Roast::Light,
        },
        Drink {
            slug: "mocha",
            name: "Mocha",
            tasting_notes: "Espresso and dark chocolate under whipped cream.",
            price: 6.0,
            roast: Roast::Dark,
        },
    ]
}
