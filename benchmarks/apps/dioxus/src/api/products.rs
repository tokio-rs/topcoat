use axum::{
    Json,
    extract::{Path, Query, rejection::QueryRejection},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use time::Date;

use super::ApiError;
use crate::{
    model::{Color, Product, iso_date},
    store::{Listing, Store},
};

/// The raw query parameters of a product listing.
#[derive(Deserialize)]
pub struct ListingParams {
    category: Option<String>,
    sort: Option<String>,
    page: Option<String>,
}

/// A page of products.
#[derive(Serialize)]
struct ProductPage<'a> {
    items: Vec<ProductSummary<'a>>,
    page: usize,
    page_count: usize,
    total: usize,
}

/// The product fields included in list responses.
#[derive(Serialize)]
struct ProductSummary<'a> {
    id: u32,
    slug: &'a str,
    name: &'a str,
    category: &'a str,
    price_cents: u32,
    compare_at_cents: Option<u32>,
    rating: f64,
    review_count: u32,
    stock: u32,
    color: Color,
    tagline: &'a str,
    #[serde(with = "iso_date")]
    created_at: Date,
    featured: bool,
}

impl<'a> From<&'a Product> for ProductSummary<'a> {
    fn from(product: &'a Product) -> Self {
        Self {
            id: product.id,
            slug: &product.slug,
            name: &product.name,
            category: &product.category,
            price_cents: product.price_cents,
            compare_at_cents: product.compare_at_cents,
            rating: product.rating,
            review_count: product.review_count,
            stock: product.stock,
            color: product.color,
            tagline: &product.tagline,
            created_at: product.created_at,
            featured: product.featured,
        }
    }
}

/// `GET /api/products`: the same query as the listing page, as JSON.
pub async fn list_products(
    query: Result<Query<ListingParams>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Ok(Query(params)) = query else {
        return Err(ApiError::invalid("invalid query"));
    };
    let listing = Listing::new(
        Store::get(),
        params.category.as_deref(),
        params.sort.as_deref(),
        params.page.as_deref(),
    )?;
    let page = ProductPage {
        items: listing.products().map(ProductSummary::from).collect(),
        page: listing.page,
        page_count: listing.page_count,
        total: listing.total,
    };
    Ok(Json(page).into_response())
}

/// `GET /api/products/{id}`: one product with all its fields.
pub async fn product(Path(id): Path<String>) -> Result<Response, ApiError> {
    let product = id
        .parse()
        .ok()
        .and_then(|id| Store::get().product(id))
        .ok_or(ApiError::not_found("product not found"))?;
    Ok(Json(product).into_response())
}
