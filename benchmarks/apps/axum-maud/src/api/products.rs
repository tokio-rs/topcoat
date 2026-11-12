use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::ApiError;
use crate::data::{Listing, ListingParams, ProductSummary, Store};

/// A page of products.
#[derive(Serialize)]
struct ProductPage<'a> {
    items: Vec<ProductSummary<'a>>,
    page: usize,
    page_count: usize,
    total: usize,
}

/// `GET /api/products`: the same query as the listing page, as JSON.
pub async fn list_products(
    State(store): State<Arc<Store>>,
    query: Result<Query<ListingParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Ok(Query(params)) = query else {
        return Err(ApiError::invalid("invalid query"));
    };
    let listing = Listing::new(&store, &params)?;
    let page = ProductPage {
        items: listing.products().map(ProductSummary::from).collect(),
        page: listing.page,
        page_count: listing.page_count,
        total: listing.total,
    };
    // Converted here because the page borrows from the store.
    Ok(Json(page).into_response())
}

/// `GET /api/products/{id}`: one product with all its fields.
pub async fn product(
    State(store): State<Arc<Store>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let product = id
        .parse()
        .ok()
        .and_then(|id| store.product(id))
        .ok_or(ApiError::not_found("product not found"))?;
    Ok(Json(product).into_response())
}
