use std::sync::Arc;

use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::ApiError;
use crate::store::Store;

/// The body of an add-to-cart request.
#[derive(Deserialize)]
pub struct CartRequest {
    product_id: i64,
    quantity: i64,
}

/// A priced cart line.
#[derive(Serialize)]
pub struct CartLine {
    product_id: u32,
    name: String,
    quantity: u32,
    unit_price_cents: u32,
    line_total_cents: u64,
}

/// `POST /api/cart`: validates a cart line and prices it. Nothing is stored.
pub async fn add_to_cart(
    State(store): State<Arc<Store>>,
    body: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Json<CartLine>, ApiError> {
    // Read the body as a JSON object first: serde also builds structs from arrays, such as
    // `[1, 2]`, and only objects are valid requests.
    let request = body
        .ok()
        .and_then(|Json(body)| CartRequest::deserialize(Value::Object(body)).ok())
        .ok_or(ApiError::invalid("invalid request body"))?;
    let product = u32::try_from(request.product_id)
        .ok()
        .and_then(|id| store.product(id))
        .ok_or(ApiError::invalid("product not found"))?;
    if product.is_sold_out() {
        return Err(ApiError::invalid("product is sold out"));
    }
    let max = product.max_quantity();
    let quantity = u32::try_from(request.quantity)
        .ok()
        .filter(|quantity| (1..=max).contains(quantity))
        .ok_or_else(|| ApiError::invalid(format!("quantity must be between 1 and {max}")))?;
    Ok(Json(CartLine {
        product_id: product.id,
        name: product.name.clone(),
        quantity,
        unit_price_cents: product.price_cents,
        line_total_cents: u64::from(product.price_cents) * u64::from(quantity),
    }))
}
