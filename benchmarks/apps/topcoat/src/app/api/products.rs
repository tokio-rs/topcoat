pub mod product_id;

use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, query_params, route},
};

use super::ApiError;
use crate::data::{Listing, ListingError, ListingParams, ProductSummary, store};

/// A page of products.
#[derive(Serialize)]
pub struct ProductPage<'a> {
    items: Vec<ProductSummary<'a>>,
    page: usize,
    page_count: usize,
    total: usize,
}

/// `GET /api/products`: the same query as the listing page, as JSON.
#[route(GET)]
pub async fn list_products(cx: &Cx) -> Result<Json<ProductPage<'_>>> {
    let params =
        query_params::<ListingParams>(cx).map_err(|_| ApiError::invalid("invalid query"))?;
    let listing = Listing::new(store(cx), params).map_err(|error| match error {
        ListingError::CategoryNotFound => ApiError::not_found("category not found"),
        ListingError::PageNotFound => ApiError::not_found("page not found"),
        ListingError::InvalidSort => ApiError::invalid("invalid sort"),
        ListingError::InvalidPage => ApiError::invalid("invalid page"),
    })?;
    Ok(Json(ProductPage {
        items: listing.products().map(ProductSummary::from).collect(),
        page: listing.page,
        page_count: listing.page_count,
        total: listing.total,
    }))
}
