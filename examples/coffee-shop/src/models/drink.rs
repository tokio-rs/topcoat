use topcoat::context::{Cx, memoize};

use super::db;

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
#[memoize]
async fn query_drinks(cx: &Cx) -> topcoat::Result<Vec<Drink>> {
    let result = Drink::all()
        .order_by(Drink::fields().menu_order().asc())
        .exec(&mut db(cx))
        .await?;
    Ok(result)
}

pub async fn drinks(cx: &Cx) -> topcoat::Result<&Vec<Drink>> {
    query_drinks(cx)
        .await
        .map_err(|error| std::io::Error::other(error.to_string()).into())
}
