use topcoat::context::{Cx, memoize};

use super::db;

/// A drink on the menu.
#[derive(Debug, toasty::Model)]
pub struct Drink {
    /// The URL segment of the drink's page, like `flat-white`.
    #[key]
    pub slug: String,
    /// The display name.
    pub name: String,
    /// A one-line description of how the drink tastes.
    pub tasting_notes: String,
    /// The price in dollars, as `f64` so runtime expressions in the browser
    /// can multiply it by a quantity signal.
    pub price: f64,
    /// The roast profile of the beans.
    pub roast: Roast,
    /// The position of the drink on the menu.
    pub(super) menu_order: i64,
}

/// The roast profile a drink is brewed from.
#[derive(Debug, Clone, Copy, toasty::Embed)]
pub enum Roast {
    Light,
    Medium,
    Dark,
}

/// Loads the menu, at most once per request.
///
/// `#[memoize]` caches the result for the duration of a request, so views can
/// share the ordered menu without issuing duplicate Toasty queries.
#[memoize(as_ref)]
async fn query_drinks(cx: &Cx) -> topcoat::Result<Vec<Drink>> {
    let result = Drink::all()
        .order_by(Drink::fields().menu_order().asc())
        .exec(&mut db(cx))
        .await?;
    Ok(result)
}

/// Returns the menu in display order.
///
/// The query runs at most once per request, so layouts, pages, and components
/// can all call this without extra database round-trips.
pub async fn drinks(cx: &Cx) -> topcoat::Result<&Vec<Drink>> {
    query_drinks(cx)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()).into())
}
