use serde::Serialize;
use topcoat::router::{Href, href};

use crate::app::products;

/// The `/products` query in its canonical parameter order (`page`, `sort`,
/// `category`), leaving out the values that are the page's defaults. Every
/// benchmark app builds identical URLs so rendered documents can be diffed.
#[derive(Serialize)]
pub struct ProductsQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'a str>,
}

/// A `/products` URL: the page as the href target, the query above behind it.
pub type ProductsUrl<'a> = Href<products::page, (), (ProductsQuery<'a>,), &'static str>;

/// Builds the URL of the products page for one page, sort order, and category.
pub fn products_url<'a>(
    page: usize,
    sort: Option<&'a str>,
    category: Option<&'a str>,
) -> ProductsUrl<'a> {
    href!(products::page).query(ProductsQuery {
        page: (page > 1).then_some(page),
        sort,
        category,
    })
}

/// Maps the raw `sort` query value onto the four supported sort orders;
/// anything else falls back to the default (ascending id) order.
pub fn normalize_sort(sort: Option<&str>) -> Option<&'static str> {
    match sort {
        Some("name") => Some("name"),
        Some("price") => Some("price"),
        Some("price-desc") => Some("price-desc"),
        Some("rating") => Some("rating"),
        _ => None,
    }
}
