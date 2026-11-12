use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, module_param, route},
};

use crate::{
    app::api::ApiError,
    data::{Product, store},
};

module_param!(product_id: u32);

/// `GET /api/products/{id}`: one product with all its fields.
#[route(GET)]
async fn product(cx: &Cx) -> Result<Json<&Product>> {
    let product = product_id(cx)
        .ok()
        .and_then(|&id| store(cx).product(id))
        .ok_or(ApiError::not_found("product not found"))?;
    Ok(Json(product))
}
