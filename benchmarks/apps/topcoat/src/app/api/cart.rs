use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use topcoat::{
    Result,
    context::Cx,
    router::{Body, content::Json, request::FromRequest, route},
};

use super::ApiError;
use crate::data::store;

/// The body of an add-to-cart request.
#[derive(Deserialize)]
struct CartRequest {
    product_id: i64,
    quantity: i64,
}

/// A priced cart line.
#[derive(Serialize)]
struct CartLine<'a> {
    product_id: u32,
    name: &'a str,
    quantity: u32,
    unit_price_cents: u32,
    line_total_cents: u64,
}

/// `POST /api/cart`: validates a cart line and prices it. Nothing is stored.
#[route(POST)]
async fn add_to_cart(cx: &Cx, body: Body) -> Result<Json<CartLine<'_>>> {
    // Parsed here rather than as an extractor so that a bad body is a 422, like every other
    // validation failure of this route. It is read as a JSON object first: serde also builds
    // structs from arrays, such as `[1, 2]`, and only objects are valid requests.
    let request = Json::<Map<String, Value>>::from_request(cx, body)
        .await
        .ok()
        .and_then(|Json(body)| CartRequest::deserialize(Value::Object(body)).ok())
        .ok_or(ApiError::invalid("invalid request body"))?;
    let product = u32::try_from(request.product_id)
        .ok()
        .and_then(|id| store(cx).product(id))
        .ok_or(ApiError::invalid("product not found"))?;
    if product.is_sold_out() {
        return Err(ApiError::invalid("product is sold out").into());
    }
    let max = product.max_quantity();
    let quantity = u32::try_from(request.quantity)
        .ok()
        .filter(|quantity| (1..=max).contains(quantity))
        .ok_or_else(|| ApiError::invalid(format!("quantity must be between 1 and {max}")))?;
    Ok(Json(CartLine {
        product_id: product.id,
        name: &product.name,
        quantity,
        unit_price_cents: product.price_cents,
        line_total_cents: u64::from(product.price_cents) * u64::from(quantity),
    }))
}
