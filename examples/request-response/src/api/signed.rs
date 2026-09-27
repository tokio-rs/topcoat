use crate::{SignedJson, User};
use topcoat::{Result, router::{content::Json, route}};

#[route(POST)]
pub(crate) async fn signed(SignedJson(user): SignedJson<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
