use topcoat::{
    Result,
    context::Cx,
    router::{content::Json, route},
};

use crate::User;

// Json<T> parses an application/json request body and serializes the response.
#[route(POST)]
pub(crate) async fn create_user(cx: &Cx, Json(user): Json<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
