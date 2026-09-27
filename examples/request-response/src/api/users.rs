use crate::User;
use topcoat::{Result, router::{content::Json, route}};

// Json<T> parses an application/json request body and serializes the response.
#[route(POST)]
pub(crate) async fn create_user(Json(user): Json<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
