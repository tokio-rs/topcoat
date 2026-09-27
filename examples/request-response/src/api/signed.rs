use topcoat::{
    Result,
    router::{content::Json, route},
};

use crate::{SignedJson, User};

#[route(POST)]
pub(crate) async fn signed(SignedJson(user): SignedJson<User>) -> Result<Json<User>> {
    Ok(Json(user))
}
